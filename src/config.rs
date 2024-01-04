use std::{ops::Deref, path::PathBuf, str::FromStr};

use clap::Parser;
use clap_serde_derive::ClapSerde;
use color_eyre::eyre::anyhow;
use lazy_static::lazy_static;
use nostr::{secp256k1::SecretKey, FromBech32};
use serde::{Deserialize, Serialize};
use url::Url;

const DEFAULT_PRIVATE_KEY: &str = "nsec1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqsmhltgl";

lazy_static! {
    pub static ref DEFAULT_CONFIG_PATH: PathBuf = PathBuf::from("nostr-relay-tester.toml");
}

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
    pub nips: Vec<Nips>,
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
    Nip09,
}

impl FromStr for Nips {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "nip01" => Ok(Nips::Nip01),
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
