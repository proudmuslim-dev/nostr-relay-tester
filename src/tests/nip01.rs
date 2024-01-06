use nostr::{secp256k1::XOnlyPublicKey, Kind, SubscriptionId, Timestamp};
use nostr_sdk::{InternalSubscriptionId, RelayPoolNotification};
use once_cell::sync::Lazy;
use tokio::sync::broadcast::Receiver;

use crate::tests::prelude::*;

static PUBLISH_TEST_INTERNAL_SUBSCRIPTION_ID: Lazy<SubscriptionId> = Lazy::new(|| SubscriptionId::new("publish_test"));

pub async fn test(client: &NostrClient, relay: &Relay) -> TestReport {
    let span = span!(Level::INFO, "nip01: publishing event").entered();

    let mut errors = vec![];

    let external_subscription: Option<(SubscriptionId, Timestamp)> =
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

    if let Some((id, timestamp)) = external_subscription {
        test_receive_published_note_from_subscription(
            client.notifications(),
            published_id,
            id.clone(),
            client.keys().await.public_key(),
            timestamp,
            &mut errors,
        )
        .await;

        match relay.send_msg(ClientMessage::Close(id.clone()), None).await {
            Ok(()) => info!("successfully closed new events subscription: {id}"),
            Err(e) => log_and_store_error(anyhow!("failed to close subscription {id}: {e}"), &mut errors),
        }
    }

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
) -> Option<(SubscriptionId, Timestamp)> {
    let timestamp = Timestamp::now();

    let subscription_result = relay
        .subscribe_with_internal_id(
            InternalSubscriptionId::Custom(PUBLISH_TEST_INTERNAL_SUBSCRIPTION_ID.to_string()),
            vec![nostr::Filter::new()
                .author(pubkey)
                .kind(Kind::TextNote)
                .since(timestamp)],
            None,
        )
        .await;

    match subscription_result {
        Ok(()) => {
            let external_id = relay
                .subscriptions()
                .await
                .get(&InternalSubscriptionId::Custom(
                    PUBLISH_TEST_INTERNAL_SUBSCRIPTION_ID.to_string(),
                ))?
                .id();

            info!("successfully established new events subscription with ID {external_id}");

            Some((external_id, timestamp))
        }
        Err(e) => {
            log_and_store_error(anyhow!("failed to create new events subscription: {e}"), errors);
            None
        }
    }
}

// TODO: Complete checks
async fn test_receive_published_note_from_subscription(
    mut notifications_stream: Receiver<RelayPoolNotification>,
    published_event_id: Option<EventId>,
    external_subscription_id: SubscriptionId,
    pubkey: XOnlyPublicKey,
    filter_since_timestamp: Timestamp,
    errors: &mut Errors,
) {
    while let Ok(notification) = notifications_stream.recv().await {
        if let RelayPoolNotification::Message {
            message: relay_message, ..
        } = notification
        {
            let relay_message = &relay_message;

            // Check subscription ID for all messages that come with one
            match relay_message {
                RelayMessage::Event { subscription_id, .. }
                | RelayMessage::Closed { subscription_id, .. }
                | RelayMessage::Count { subscription_id, .. }
                | RelayMessage::EndOfStoredEvents(subscription_id) => {
                    // TODO: Check if nostr-sdk lets messages with unknown subscription IDs through.
                    if !subscription_id.eq(&external_subscription_id) {
                        log_and_store_error(anyhow!("received message with subscription id {subscription_id} (expected {external_subscription_id}): {relay_message:#?}"), errors);
                    }
                }
                _ => {}
            }

            // Perform checks unique to each message type
            match relay_message {
                RelayMessage::Event { event, .. } => {
                    if matches!(published_event_id, Some(id) if id.eq(&event.id)) {
                        info!("received published event from subscription: {event:#?}");
                        return;
                    }

                    if event.kind != Kind::TextNote {
                        log_and_store_error(anyhow!("received event of kind other than 1: {event:#?}"), errors);
                    }

                    if event.pubkey != pubkey {
                        log_and_store_error(
                            anyhow!(
                                "received event with author {} (expected {pubkey}): {event:#?}",
                                event.pubkey
                            ),
                            errors,
                        );
                    }

                    if event.created_at < filter_since_timestamp {
                        log_and_store_error(
                            anyhow!(
                                "Received event with timestamp {} (expected >= {filter_since_timestamp}): {event:#?}",
                                event.created_at
                            ),
                            errors,
                        );
                    }
                }
                RelayMessage::Closed {
                    subscription_id,
                    message,
                } => {
                    log_and_store_error(
                        anyhow!("relay closed subscription \"{subscription_id}\" unexpectedly: {message}"),
                        errors,
                    );

                    return;
                }
                RelayMessage::Notice { message } => warn!("received NOTICE event from relay: {message}"),
                RelayMessage::Ok { .. } => todo!(),
                RelayMessage::EndOfStoredEvents(subscription_id) => {
                    info!("received EOSE event for subscription: {subscription_id}")
                }
                RelayMessage::Count { subscription_id, count } => log_and_store_error(
                    anyhow!("received unexpected COUNT event for subscription {subscription_id}: {count}"),
                    errors,
                ),
                _ => {}
            }
        }
    }
}
