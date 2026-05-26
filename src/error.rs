use thiserror::Error;

#[derive(Error, Debug)]
pub enum LacamError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: failed to parse {field}")]
    Parse {field: String}
}

pub type Result<T> = std::result::Result<T, LacamError>;