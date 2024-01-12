mod logger;
pub mod report;

pub mod nip01;
pub mod nip02;
pub mod nip09;

use color_eyre::eyre;

use crate::{config::Config, tests::report::TestReport};

pub async fn run(Config { key, nips, relay_url }: Config) -> eyre::Result<Vec<TestReport>> {
    use crate::NostrClient;
    use eyre::anyhow;
    use nostr_sdk::client::Options as NostrClientOptions;

    let relay_url = relay_url.ok_or(anyhow!("Relay URL must be specified!"))?;

    let client = {
        let options = NostrClientOptions::new()
            .wait_for_connection(true)
            .wait_for_send(true)
            .wait_for_subscription(true)
            .shutdown_on_drop(true);

        NostrClient::with_opts(&key, options)
    };
    client.add_relay(relay_url.as_str()).await?;
    client.connect().await;

    let relay = client.relay(relay_url.as_str()).await?;
    let mut results = vec![];

    for nip in nips {
        results.push(nip.test(&client, &relay).await);
    }

    Ok(results)
}

mod prelude {
    pub use crate::{
        config::Nips,
        tests::{
            logger::{LogEvent, Logger},
            report::{Errors, TestReport},
        },
        NostrClient,
    };
    pub use color_eyre::eyre::anyhow;
    pub use nostr::{ClientMessage, EventId, Kind, RelayMessage, SubscriptionId, Timestamp};
    pub use nostr_sdk::{InternalSubscriptionId, Relay, RelayPoolNotification};
    pub use once_cell::sync::Lazy;
    pub use tracing::{span, Level};

    /// Adds timestamp constraint to the filter and establishes a subscription,
    /// returning its external ID and the aforementioned timestamp if the
    /// subscription was successfully established.
    pub(super) async fn establish_subscription(
        name: &str,
        relay: &Relay,
        internal_id: InternalSubscriptionId,
        filter: nostr::Filter,
        logger: &mut Logger,
    ) -> Option<(SubscriptionId, Timestamp)> {
        let timestamp = Timestamp::now();

        let subscription_result = relay
            .subscribe_with_internal_id(internal_id.clone(), vec![filter.since(timestamp)], None)
            .await;

        match subscription_result {
            Ok(()) => {
                let external_id = relay.subscription(&internal_id).await?.id();

                logger.log(LogEvent::EstablishedSubscription(name, &external_id));

                Some((external_id, timestamp))
            }
            Err(error) => {
                logger.log(LogEvent::FailedToEstablishSubscription(name, &error));
                None
            }
        }
    }

    pub(super) async fn close_subscription(name: &str, relay: &Relay, id: SubscriptionId, logger: &mut Logger) {
        match relay.send_msg(ClientMessage::Close(id.clone()), None).await {
            Ok(()) => logger.log(LogEvent::ClosedSubscription(name, &id)),
            Err(error) => logger.log(LogEvent::FailedToCloseSubscription(name, &id, &error)),
        }
    }
}
