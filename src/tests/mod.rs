pub mod report;

pub mod nip01;
pub mod nip09;

use color_eyre::eyre;

fn log_and_store_error(err: eyre::Error, errors_vec: &mut Vec<eyre::Error>) {
    tracing::error!("{err}");
    errors_vec.push(err);
}
