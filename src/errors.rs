use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;

pub type RhodResult<T> = Result<T, RhodError>;

// Represents an error while handling a request/response.
// Used to communication between Handlers.
// RhodErrors are thrown by users of the library in their implementations of handle_request/handle_response/serve functions

#[derive(Debug)]
pub enum RhodErrorLevel {
    Debug,
    Warning,
    Error,
    Critical,
}

#[derive(Debug)]
pub struct RhodError {
    msg: String,
    level: RhodErrorLevel,
}

impl RhodError {
    pub fn from_string(msg: String, level: RhodErrorLevel) -> RhodError {
        RhodError { msg, level }
    }

    pub fn from_str(msg: &str, level: RhodErrorLevel) -> RhodError {
        RhodError {
            msg: String::from(msg),
            level,
        }
    }

    pub fn log(&self) {
        match self.level {
            RhodErrorLevel::Warning => warn!("{}", self),
            RhodErrorLevel::Error => error!("{}", self),
            RhodErrorLevel::Critical => error!("{}", self),
            RhodErrorLevel::Debug => info!("{}", self),
        }
    }
}

impl Display for RhodError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for RhodError {}

//Represents server errors (Hyper errors, bad certificates, etc)
#[derive(Debug)]
pub enum RhodHyperError {
    HyperError(hyper::Error),
    ConfigError(String),
}

impl RhodHyperError {
    pub fn from_hyper_error_result(result: Result<(), hyper::Error>) -> Result<(), RhodHyperError> {
        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(RhodHyperError::HyperError(e)),
        }
    }
}

impl Display for RhodHyperError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self {
            RhodHyperError::HyperError(e) => write!(f, "HYPER ERROR: {}", e),
            RhodHyperError::ConfigError(e) => write!(f, "CONFIG ERROR: {}", e),
        }
    }
}

impl std::error::Error for RhodHyperError {}
