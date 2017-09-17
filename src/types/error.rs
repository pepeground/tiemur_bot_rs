error_chain! {
    foreign_links {
        RocksdbError(::rocksdb::Error);
        HyperUriError(::hyper::error::UriError);
        HyperError(::hyper::Error);
        ImageError(::image::ImageError);
        BincodeError(::bincode::Error);
    }

    links {
        Telegram(::telegram_bot::Error, ::telegram_bot::ErrorKind);
    }
}

impl From<Error> for ::telegram_bot::Error {
    fn from(e: Error) -> ::telegram_bot::Error {
        e.to_string().into()
    }
}
