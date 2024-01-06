pub mod report;

pub mod nip01;
pub mod nip09;

use color_eyre::eyre;

use crate::{config::Config, tests::report::TestReport};

pub async fn run(Config { key, nips, relay_url }: Config) -> eyre::Result<Vec<TestReport>> {
    use crate::NostrClient;
    use eyre::anyhow;

    let relay_url = relay_url.ok_or(anyhow!("Relay URL must be specified!"))?;

    let client = NostrClient::new(&key);
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
    use color_eyre::eyre;

    pub use crate::{
        config::Nips,
        tests::report::{Errors, TestReport},
        NostrClient,
    };
    pub use color_eyre::eyre::anyhow;
    pub use nostr::{ClientMessage, EventId, RelayMessage};
    pub use nostr_sdk::{Relay, RelayPoolNotification};
    pub use tracing::{info, span, warn, Level};

    pub(super) fn log_and_store_error(err: eyre::Error, errors_vec: &mut Vec<eyre::Error>) {
        tracing::error!("{err}");
        errors_vec.push(err);
    }
}
