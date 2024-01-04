use clap::Parser;
use clap_serde_derive::ClapSerde;
use color_eyre::eyre::anyhow;
use nostr_relay_tester::{
    config::{CliArgs, Config, DEFAULT_CONFIG_PATH},
    tests::report::TestReport,
    NostrClient,
};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::warn;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let mut args = CliArgs::parse();

    let Config { key, nips, relay_url } = match File::open(&args.config_path).await {
        Ok(mut f) => {
            let mut cfg_str = "".to_owned();
            f.read_to_string(&mut cfg_str).await?;

            let c = toml::from_str::<<Config as ClapSerde>::Opt>(cfg_str.as_str())?;

            Config::from(c).merge(&mut args.config)
        }
        Err(e) if args.config_path.eq(&*DEFAULT_CONFIG_PATH) => {
            warn!("Error accessing default config file: {e}");
            Config::from(&mut args.config)
        }
        Err(e) => Err(e)?,
    };

    let relay_url = relay_url.ok_or(anyhow!("Relay URL must be specified!"))?;

    let client = NostrClient::new(&key);
    client.add_relay(relay_url.as_str()).await?;
    client.connect().await;

    let relay = client.relay(relay_url.as_str()).await?;
    let mut results: Vec<TestReport> = vec![];

    for nip in nips {
        results.push(nip.test(&client, &relay).await);
    }

    Ok(())
}
