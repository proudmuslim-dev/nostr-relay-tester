use std::fmt::Display;

use crate::config::Nips;

type Nip = Nips;

pub type Errors = Vec<color_eyre::eyre::Error>;

pub enum TestReport {
    Passed(Nip),
    Failed { nip: Nip, errors: Errors },
}

impl Display for TestReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("Pretty print report")
    }
}
