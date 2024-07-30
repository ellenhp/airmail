use thiserror::Error;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("unable to import OSMX file")]
    OsmxImport,

    #[error("no names found")]
    NoNamesFound,

    #[error("no langs found")]
    NoLangsFound,

    #[error("node missing location")]
    NodeMissingLocation,
}

impl From<Box<dyn std::error::Error>> for IndexerError {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        log::error!("IndexerError::OsmxImport: {}", error);
        IndexerError::OsmxImport
    }
}
