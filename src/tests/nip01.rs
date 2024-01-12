use nostr::secp256k1::XOnlyPublicKey;
use nostr_sdk::RelayPoolNotification;
use tokio::sync::broadcast::Receiver;

use crate::tests::prelude::*;

const SUBSCRIPTION_NAME: &str = "new events";

static PUBLISH_TEST_INTERNAL_SUBSCRIPTION_ID: Lazy<InternalSubscriptionId> =
    Lazy::new(|| InternalSubscriptionId::Custom("publish_test".to_owned()));

pub async fn test(client: &NostrClient, relay: &Relay) -> TestReport {
    let span = span!(Level::INFO, "nip01: publishing event").entered();

    let mut logger = Logger::new(Nips::Nip01);

    let event_subscription: Option<(SubscriptionId, Timestamp)> = establish_subscription(
        SUBSCRIPTION_NAME,
        relay,
        PUBLISH_TEST_INTERNAL_SUBSCRIPTION_ID.clone(),
        nostr::Filter::new()
            .author(client.keys().await.public_key())
            .kind(Kind::TextNote),
        &mut logger,
    )
    .await;

    let published_id = match client.publish_text_note("nostr-relay-tester: nip01", []).await {
        Ok(id) => {
            logger.log(LogEvent::PublishedEvent(&id));
            Some(id)
        }
        Err(error) => {
            logger.log(LogEvent::FailedToPublishEvent(&error));
            None
        }
    };

    if let Some((id, timestamp)) = event_subscription {
        if let Some(event_id) = published_id {
            test_receive_published_note_from_subscription(
                SUBSCRIPTION_NAME,
                client.notifications(),
                event_id,
                id.clone(),
                client.keys().await.public_key(),
                timestamp,
                &mut logger,
            )
            .await;

            // TODO: Fetch event from relay
        }

        close_subscription(SUBSCRIPTION_NAME, relay, id, &mut logger).await;
    }

    drop(span);

    // TODO: Set metadata
    let span = span!(Level::INFO, "nip01: set metadata").entered();

    drop(span);

    TestReport::from(logger)
}

// TODO: Extract this into a listener function that takes in a desired check
// TODO: Some kind of timeout in case the relay never sends the desired event
async fn test_receive_published_note_from_subscription(
    subscription_name: &str,
    mut notifications_stream: Receiver<RelayPoolNotification>,
    published_event_id: EventId,
    external_subscription_id: SubscriptionId,
    pubkey: XOnlyPublicKey,
    filter_since_timestamp: Timestamp,
    logger: &mut Logger,
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
                        logger.log(LogEvent::UnknownSubscriptionId {
                            expected_id: &external_subscription_id,
                            id: subscription_id,
                            message: relay_message,
                        });
                    }
                }
                _ => {}
            }

            // Perform checks unique to each message type
            match relay_message {
                RelayMessage::Event { event, .. } => {
                    if published_event_id.eq(&event.id) {
                        return logger.log(LogEvent::ReceivedExpectedEvent("published", event));
                    }

                    if event.kind != Kind::TextNote {
                        logger.log(LogEvent::BadEventKind {
                            expected_kind: Kind::TextNote,
                            event,
                        });
                    }

                    if event.pubkey != pubkey {
                        logger.log(LogEvent::BadEventAuthor {
                            expected_author: &pubkey,
                            author: &event.pubkey,
                            event,
                        });
                    }

                    if event.created_at < filter_since_timestamp {
                        logger.log(LogEvent::BadEventTimestamp {
                            filter_since_timestamp,
                            event_timestamp: event.created_at,
                            event,
                        });
                    }
                }
                RelayMessage::Closed {
                    subscription_id,
                    message,
                } => {
                    if subscription_id.eq(&external_subscription_id) {
                        return logger.log(LogEvent::UnexpectedlyClosedSubscription {
                            name: subscription_name,
                            id: &external_subscription_id,
                            message,
                        });
                    }

                    logger.log(LogEvent::UnknownSubscriptionClosed(subscription_id, message));
                }
                RelayMessage::Notice { message } => logger.log(LogEvent::ReceivedNoticeEvent(message)),
                RelayMessage::Ok { event_id, .. } => {
                    // Check should be redundant, but it's placed here in case it's not.
                    if published_event_id.ne(event_id) {
                        logger.log(LogEvent::UnexpectedOkEvent(event_id, relay_message));
                    }
                }

                RelayMessage::EndOfStoredEvents(subscription_id) => {
                    logger.log(LogEvent::ReceivedEndOfStoredEvents(subscription_id));
                }
                RelayMessage::Count { subscription_id, count } => {
                    logger.log(LogEvent::UnexpectedCountEvent(subscription_id, *count))
                }
                _ => {}
            }
        }
    }
}
