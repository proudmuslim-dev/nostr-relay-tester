use clap::Parser;
use clap_serde_derive::ClapSerde;
use color_eyre::eyre::anyhow;
use nostr_relay_tester::{
    config::{CliArgs, Config, Nips, DEFAULT_CONFIG_PATH},
    NostrClient,
};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::warn;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let mut args = CliArgs::parse();
    let config = match File::open(&args.config_path).await {
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

    if config.relay_url.is_none() {
        return Err(anyhow!("Relay URL must be specified!"));
    }

    let client = NostrClient::new(&config.key);

    client.add_relay(config.relay_url.unwrap().as_str()).await?;
    client.connect().await;

    for nip in config.nips {
        use nostr_relay_tester::{nip01, nip09};
        match nip {
            Nips::Nip01 => nip01::test(&client).await,
            Nips::Nip09 => nip09::test(&client).await,
        }
    }

    Ok(())
}
