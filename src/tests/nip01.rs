use color_eyre::eyre::anyhow;
use nostr::{secp256k1::XOnlyPublicKey, EventId, Kind, RelayMessage, SubscriptionId, Timestamp};
use nostr_sdk::{Client as NostrClient, InternalSubscriptionId, Relay, RelayPoolNotification};
use once_cell::sync::Lazy;
use tokio::sync::broadcast::Receiver;
use tracing::{info, span, Level};

use crate::{
    config::Nips,
    tests::{
        log_and_store_error,
        report::{Errors, TestReport},
    },
};

static PUBLISH_TEST_SUBSCRIPTION_ID: Lazy<SubscriptionId> = Lazy::new(|| SubscriptionId::new("publish_test"));

pub async fn test(client: &NostrClient, relay: &Relay) -> TestReport {
    let span = span!(Level::INFO, "nip01: publishing event");
    let _s = span.enter();

    let mut errors = vec![];

    test_establish_publish_subscription(relay, client.keys().await.public_key(), &mut errors).await;

    let published_id = match client.publish_text_note("nostr-relay-tester: nip01", vec![]).await {
        Ok(id) => {
            info!("successfully published: {id}");
            Some(id)
        }
        Err(e) => {
            log_and_store_error(anyhow!("failed to publish event: {e}"), &mut errors);
            None
        }
    };

    test_receive_published_note_from_subscription(published_id, client.notifications(), &mut errors).await;

    relay
        .send_msg(nostr::ClientMessage::Close(PUBLISH_TEST_SUBSCRIPTION_ID.clone()), None)
        .await;

    drop(_s);
    drop(span);

    if !errors.is_empty() {
        return TestReport::Failed {
            nip: Nips::Nip01,
            errors,
        };
    }

    TestReport::Passed(Nips::Nip01)
}

async fn test_establish_publish_subscription(relay: &Relay, pubkey: XOnlyPublicKey, errors: &mut Errors) {
    let subscription_result = relay
        .subscribe_with_internal_id(
            InternalSubscriptionId::Custom(PUBLISH_TEST_SUBSCRIPTION_ID.to_string()),
            vec![nostr::Filter::new()
                .pubkey(pubkey)
                .kind(Kind::TextNote)
                .since(Timestamp::now())],
            None,
        )
        .await;

    match subscription_result {
        Ok(()) => info!("successfully established new event subscription"),
        Err(e) => {
            log_and_store_error(anyhow!("failed to create new events subscription: {e}"), errors);
        }
    }
}

// TODO: Complete checks
async fn test_receive_published_note_from_subscription(
    published_id: Option<EventId>,
    mut notifications_stream: Receiver<RelayPoolNotification>,
    errors: &mut Errors,
) {
    while let Ok(notification) = notifications_stream.recv().await {
        if let RelayPoolNotification::Message { message, .. } = notification {
            let mut check_subscription_id = |subscription_id: &SubscriptionId| {
                if !subscription_id.eq(&PUBLISH_TEST_SUBSCRIPTION_ID) {
                    log_and_store_error(anyhow!("incorrect subscription id: {subscription_id}"), errors);
                }
            };

            match message {
                RelayMessage::Event { subscription_id, event } => {
                    check_subscription_id(&subscription_id);

                    if matches!(published_id, Some(id) if id.eq(&event.id)) {
                        info!("received published event from subscription: {event:#?}");
                        break;
                    }

                    if event.kind != Kind::TextNote {
                        log_and_store_error(anyhow!("received event of kind other than 1: {event:#?}"), errors);
                    }
                }
                RelayMessage::Closed {
                    subscription_id,
                    message,
                } => {
                    check_subscription_id(&subscription_id);
                    log_and_store_error(
                        anyhow!("relay closed subscription \"{subscription_id}\" unexpectedly: {message}"),
                        errors,
                    );
                }
                RelayMessage::Notice { message } => todo!(),
                RelayMessage::Ok { .. } => todo!(),
                RelayMessage::Count { subscription_id, .. } | RelayMessage::EndOfStoredEvents(subscription_id) => {
                    todo!()
                }
                _ => {}
            }
        }
    }
}
