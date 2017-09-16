use rocksdb::Error as RocksdbError;
use telegram_bot::Error as TelegramError;
use hyper::error::UriError;
use hyper::Error as HyperError;
use image::ImageError;

pub enum Error {
    TelegramError(TelegramError),
    RocksdbError(RocksdbError),
    StringError(String),
    HyperUriError(UriError),
    HyperError(HyperError),
    ImageError(ImageError),
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
