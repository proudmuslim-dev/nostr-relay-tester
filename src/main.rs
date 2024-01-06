use clap::Parser;
use clap_serde_derive::ClapSerde;
use nostr_relay_tester::config::{CliArgs, Config, DEFAULT_CONFIG_PATH};
use tokio::{fs::File, io::AsyncReadExt};
use tracing::warn;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let config = {
        let mut args = CliArgs::parse();

        match File::open(&args.config_path).await {
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
        }
    };

    nostr_relay_tester::run(config)
        .await?
        .iter()
        .for_each(|result| println!("{result}"));

    Ok(())
}
