use thiserror::Error;

#[derive(Error, Debug)]
pub enum AirmailError {
    #[error("unable to count")]
    UnableToCount,
}
