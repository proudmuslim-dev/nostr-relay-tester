use std::{ops::Deref, path::PathBuf, str::FromStr};

use crate::{tests::report::TestReport, NostrClient};
use clap::Parser;
use clap_serde_derive::ClapSerde;
use color_eyre::eyre::anyhow;
use nostr::{secp256k1::SecretKey, FromBech32};
use nostr_sdk::Relay;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use url::Url;

const DEFAULT_PRIVATE_KEY: &str = "nsec1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsmhltgl";

pub static DEFAULT_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| PathBuf::from("nostr-relay-tester.toml"));

#[derive(Parser)]
#[command(author, version, about)]
pub struct CliArgs {
    /// Config file
    #[arg(short, long = "config", default_value = DEFAULT_CONFIG_PATH.as_os_str())]
    pub config_path: PathBuf,

    /// Rest of arguments
    #[command(flatten)]
    pub config: <Config as ClapSerde>::Opt,
}

#[derive(ClapSerde)]
pub struct Config {
    #[arg(
        short,
        long,
        help = "Must be specified in the config file or as a CLI arg [example: wss://relay.primal.net]"
    )]
    pub relay_url: Option<Url>,
    // Specified here so that it is documented in --help. [`Default`] implementation is still
    // required due to annoying trait bounds
    #[arg(short, long, help = "64-character bech32-encoded private key", default_value = DEFAULT_PRIVATE_KEY)]
    pub key: NostrKeys,
    #[arg(short, long, value_delimiter = ',', default_value = "nip01,nip09")]
    pub nips: Vec<Nips>, // Cannot make this a HashSet due to trait bounds
}

#[derive(Clone)]
pub struct NostrKeys(pub nostr::Keys);

impl Default for NostrKeys {
    fn default() -> Self {
        NostrKeys(nostr::Keys::new(SecretKey::from_bech32(DEFAULT_PRIVATE_KEY).unwrap()))
    }
}

impl FromStr for NostrKeys {
    type Err = nostr::nips::nip19::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let secret_key = SecretKey::from_bech32(s)?;

        Ok(NostrKeys(nostr::Keys::new(secret_key)))
    }
}

impl Deref for NostrKeys {
    type Target = nostr::Keys;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NostrKeys {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(deser::NostrKeysVisitor)
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
#[serde(rename_all = "lowercase")]
/// Supported NIPs
pub enum Nips {
    Nip01,
    Nip02,
    Nip09,
}

impl Nips {
    pub async fn test(&self, client: &NostrClient, relay: &Relay) -> TestReport {
        /// If you hate this, blame [Tricked](https://github.com/Tricked-dev/) for encouraging me
        macro_rules! match_and_test {
            ($($number:literal )*) => {
                paste::paste! {
                    match self {
                        $(
                            Nips::[<Nip $number>] => crate::tests::[<nip $number>]::test(client, relay).await,
                        )*
                    }
                }
            }
        }

        match_and_test!(01 02 09)
    }
}

impl FromStr for Nips {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "nip01" => Ok(Nips::Nip01),
            "nip02" => Ok(Nips::Nip02),
            "nip09" => Ok(Nips::Nip09),
            _ => Err(anyhow!("Not a supported NIP: {s}")),
        }
    }
}

mod deser {
    use std::str::FromStr;

    use serde::de::{self, Visitor};

    use super::NostrKeys;

    pub struct NostrKeysVisitor;

    impl<'de> Visitor<'de> for NostrKeysVisitor {
        type Value = NostrKeys;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a 64-character bech32-encoded nostr private key")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            NostrKeys::from_str(v).map_err(|_| E::custom(format!("Invalid key: {v}")))
        }
    }
}
