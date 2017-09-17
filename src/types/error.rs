use rocksdb::Error as RocksdbError;
use telegram_bot::Error as TelegramError;
use hyper::error::UriError;
use hyper::Error as HyperError;
use image::ImageError;
use bincode::ErrorKind;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum Error {
    TelegramError(TelegramError),
    RocksdbError(RocksdbError),
    StringError(String),
    HyperUriError(UriError),
    HyperError(HyperError),
    ImageError(ImageError),
    BincodeError(ErrorKind),
}

impl From<TelegramError> for Error {
    fn from(error: TelegramError) -> Error {
        Error::TelegramError(error)
    }
}

impl From<RocksdbError> for Error {
    fn from(error: RocksdbError) -> Error {
        Error::RocksdbError(error)
    }
}

impl From<String> for Error {
    fn from(error: String) -> Error {
        Error::StringError(error)
    }
}

impl From<UriError> for Error {
    fn from(error: UriError) -> Error {
        Error::HyperUriError(error)
    }
}

impl From<HyperError> for Error {
    fn from(error: HyperError) -> Error {
        Error::HyperError(error)
    }
}

impl From<ImageError> for Error {
    fn from(error: ImageError) -> Error {
        Error::ImageError(error)
    }
}

impl From<Box<ErrorKind>> for Error {
    fn from(error: Box<ErrorKind>) -> Error {
        Error::BincodeError(*error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            &Error::TelegramError(ref error) => write!(f, "{}", error),
            &Error::RocksdbError(ref error) => write!(f, "{}", error),
            &Error::StringError(ref error) => write!(f, "{}", error),
            &Error::HyperUriError(ref error) => write!(f, "{}", error),
            &Error::HyperError(ref error) => write!(f, "{}", error),
            &Error::ImageError(ref error) => write!(f, "{}", error),
            &Error::BincodeError(ref error) => write!(f, "{}", error),
        }
    }
}
