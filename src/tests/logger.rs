use color_eyre::{eyre, eyre::anyhow};
use nostr::{secp256k1::XOnlyPublicKey, Event, EventId, Kind, RelayMessage, SubscriptionId, Timestamp};
use tracing::{error, info, warn};

use crate::{
    config::Nips,
    tests::report::{Errors, TestReport},
};

pub struct Logger {
    nip: Nips,
    errors: Errors,
}

impl Logger {
    pub fn new(nip: Nips) -> Logger {
        Logger { nip, errors: vec![] }
    }

    /// Sends event to stdout. Stores it afterwards if it's an error.
    pub fn log(&mut self, event: LogEvent) {
        match event {
            LogEvent::EstablishedSubscription(name, id) => {
                info!("successfully established {name} subscription with id {id}");
            }
            LogEvent::FailedToEstablishSubscription(name, error) => {
                self.print_and_store_error(anyhow!("failed to create {name} subscription: {error}"));
            }
            LogEvent::ClosedSubscription(name, id) => info!("successfully closed {name} subscription: {id}"),
            LogEvent::FailedToCloseSubscription(name, id, error) => {
                self.print_and_store_error(anyhow!("failed to close {name} subscription \"{id}\": {error}"));
            }
            LogEvent::PublishedEvent(event_id) => info!("successfully published event: {event_id}"),
            LogEvent::FailedToPublishEvent(client_error) => {
                self.print_and_store_error(anyhow!("failed to publish event: {client_error}"));
            }
            LogEvent::ReceivedExpectedEvent(name, event) => {
                info!("received {name} event from subscription: {event:#?}");
            }
            LogEvent::ReceivedEndOfStoredEvents(subscription_id) => {
                info!("received EOSE event for subscription: {subscription_id}");
            }
            LogEvent::ReceivedNoticeEvent(message) => warn!("received NOTICE event from relay: {message}"),
            LogEvent::UnexpectedOkEvent(event_id, relay_message) => self.print_and_store_error(anyhow!(
                "received unexpected OK event for event \"{event_id}\": {relay_message:#?}"
            )),
            LogEvent::UnexpectedCountEvent(subscription_id, count) => self.print_and_store_error(anyhow!(
                "received unexpected COUNT event for subscription \"{subscription_id}\": {count}"
            )),
            LogEvent::UnknownSubscriptionClosed(id, message) => {
                self.print_and_store_error(anyhow!("relay closed unknown subscription \"{id}\": {message}"));
            }
            LogEvent::UnknownSubscriptionId {
                expected_id,
                id,
                message,
            } => self.print_and_store_error(anyhow!(
                "received message with subscription id {id} (expected {expected_id}): {message:#?}"
            )),
            LogEvent::BadEventAuthor {
                expected_author,
                author,
                event,
            } => self.print_and_store_error(anyhow!(
                "received event with author {author} (expected {expected_author}): {event:#?}"
            )),
            LogEvent::BadEventTimestamp {
                filter_since_timestamp,
                event_timestamp,
                event,
            } => self.print_and_store_error(anyhow!(
                "Received event with timestamp {event_timestamp} (expected >= {filter_since_timestamp}): {event:#?}"
            )),
            LogEvent::BadEventKind { expected_kind, event } => self.print_and_store_error(anyhow!(
                "received event of kind {} (expected {}): {event:#?}",
                event.kind.as_u64(),
                expected_kind.as_u64(),
            )),
            LogEvent::UnexpectedlyClosedSubscription { name, id, message } => self.print_and_store_error(anyhow!(
                "relay closed {name} subscription \"{id}\" unexpectedly: {message}"
            )),
        }
    }

    fn print_and_store_error(&mut self, err: eyre::Error) {
        error!("{err}");
        self.errors.push(err);
    }
}

impl From<Logger> for TestReport {
    fn from(value: Logger) -> Self {
        if value.errors.is_empty() {
            return TestReport::Passed(value.nip);
        }

        TestReport::Failed {
            nip: value.nip,
            errors: value.errors,
        }
    }
}

#[derive(Copy, Clone)]
pub enum LogEvent<'a> {
    EstablishedSubscription(&'a str, &'a SubscriptionId),
    FailedToEstablishSubscription(&'a str, &'a nostr_sdk::relay::Error),
    ClosedSubscription(&'a str, &'a SubscriptionId),
    FailedToCloseSubscription(&'a str, &'a SubscriptionId, &'a nostr_sdk::relay::Error),
    PublishedEvent(&'a EventId),
    FailedToPublishEvent(&'a nostr_sdk::client::Error),
    ReceivedExpectedEvent(&'a str, &'a Event),
    ReceivedEndOfStoredEvents(&'a SubscriptionId),
    ReceivedNoticeEvent(&'a str),
    UnexpectedOkEvent(&'a EventId, &'a RelayMessage),
    UnexpectedCountEvent(&'a SubscriptionId, usize),
    UnknownSubscriptionClosed(&'a SubscriptionId, &'a str),
    UnknownSubscriptionId {
        expected_id: &'a SubscriptionId,
        id: &'a SubscriptionId,
        message: &'a RelayMessage,
    },
    BadEventAuthor {
        expected_author: &'a XOnlyPublicKey,
        author: &'a XOnlyPublicKey,
        event: &'a Event,
    },
    /// Event timestamp is lower than the since field in your filter.
    BadEventTimestamp {
        filter_since_timestamp: Timestamp,
        event_timestamp: Timestamp,
        event: &'a Event,
    },
    BadEventKind {
        expected_kind: Kind,
        event: &'a Event,
    },
    UnexpectedlyClosedSubscription {
        name: &'a str,
        id: &'a SubscriptionId,
        message: &'a str,
    },
}
