//! Define a general purpose ATDomeError struct.

use kafka::error as kafka_error;
use regex::Error as RegexError;
use salobj::error::errors::SalObjError;
use std::{
    error::Error,
    fmt::{self, Debug},
    result,
};

pub type ATDomeResult<T> = result::Result<T, ATDomeError>;

#[derive(Debug)]
pub struct ATDomeError {
    err_msg: String,
}

impl Error for ATDomeError {}

impl fmt::Display for ATDomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err_msg = self.err_msg.clone();
        write!(f, "ATDomeError::{err_msg}")
    }
}
impl ATDomeError {
    pub fn new(err_msg: &str) -> ATDomeError {
        ATDomeError {
            err_msg: String::from(err_msg),
        }
    }

    pub fn from_error(error: impl Error) -> ATDomeError {
        ATDomeError {
            err_msg: error.to_string(),
        }
    }

    pub fn get_error_message(&self) -> &str {
        &self.err_msg
    }
}

impl From<Box<dyn Error>> for ATDomeError {
    fn from(item: Box<dyn Error>) -> ATDomeError {
        ATDomeError::new(&item.to_string())
    }
}

impl From<SalObjError> for ATDomeError {
    fn from(item: SalObjError) -> ATDomeError {
        ATDomeError::new(&item.to_string())
    }
}

impl From<kafka_error::Error> for ATDomeError {
    fn from(item: kafka_error::Error) -> ATDomeError {
        ATDomeError::new(&item.to_string())
    }
}

impl From<RegexError> for ATDomeError {
    fn from(item: RegexError) -> ATDomeError {
        ATDomeError::new(&item.to_string())
    }
}

impl From<std::io::Error> for ATDomeError {
    fn from(item: std::io::Error) -> ATDomeError {
        ATDomeError::new(&item.to_string())
    }
}
