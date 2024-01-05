use color_eyre::eyre::anyhow;
use nostr::{
    secp256k1::XOnlyPublicKey, ClientMessage, EventId, JsonUtil, Kind, RelayMessage, SubscriptionId, Timestamp,
};
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

static PUBLISH_TEST_INTERNAL_SUBSCRIPTION_ID: Lazy<SubscriptionId> = Lazy::new(|| SubscriptionId::new("publish_test"));

pub async fn test(client: &NostrClient, relay: &Relay) -> TestReport {
    let span = span!(Level::INFO, "nip01: publishing event");
    let _s = span.enter();

    let mut errors = vec![];

    let external_subscription_id =
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

    if let Some(id) = external_subscription_id {
        let notif_stream = client.notifications();
        test_receive_published_note_from_subscription(published_id, id.clone(), notif_stream, &mut errors).await;

        relay.send_msg(ClientMessage::Close(id), None).await;
    }

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

async fn test_establish_publish_subscription(
    relay: &Relay,
    pubkey: XOnlyPublicKey,
    errors: &mut Errors,
) -> Option<SubscriptionId> {
    let subscription_result = relay
        .subscribe_with_internal_id(
            InternalSubscriptionId::Custom(PUBLISH_TEST_INTERNAL_SUBSCRIPTION_ID.to_string()),
            vec![nostr::Filter::new()
                .author(pubkey)
                .kind(Kind::TextNote)
                .since(Timestamp::now())],
            None,
        )
        .await;

    match subscription_result {
        Ok(()) => {
            info!("successfully established new event subscription");
            let external_id = relay
                .subscriptions()
                .await
                .get(&InternalSubscriptionId::Custom(
                    PUBLISH_TEST_INTERNAL_SUBSCRIPTION_ID.to_string(),
                ))?
                .id();

            Some(external_id)
        }
        Err(e) => {
            log_and_store_error(anyhow!("failed to create new events subscription: {e}"), errors);
            None
        }
    }
}

// TODO: Complete checks
async fn test_receive_published_note_from_subscription(
    published_event_id: Option<EventId>,
    external_subscription_id: SubscriptionId,
    mut notifications_stream: Receiver<RelayPoolNotification>,
    errors: &mut Errors,
) {
    while let Ok(notification) = notifications_stream.recv().await {
        if let RelayPoolNotification::Message { message, .. } = notification {
            let mut check_subscription_id = |subscription_id: &SubscriptionId| {
                if !subscription_id.eq(&external_subscription_id) {
                    log_and_store_error(anyhow!("incorrect subscription id: {subscription_id}"), errors);
                }
            };

            match message {
                RelayMessage::Event { subscription_id, event } => {
                    check_subscription_id(&subscription_id);

                    if matches!(published_event_id, Some(id) if id.eq(&event.id)) {
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
