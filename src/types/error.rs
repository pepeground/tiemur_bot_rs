error_chain! {
    foreign_links {
        HyperUriError(::hyper::error::UriError);
        HyperError(::hyper::Error);
        ImageError(::image::ImageError);
        BincodeError(::bincode::Error);
    }

    links {
        Telegram(::telegram_bot::Error, ::telegram_bot::ErrorKind);
    }

    errors {
        CasError(v: Vec<u8>) {
            description("compare and swap return error result")
            display("compare and swap return {:?}", v)
        }
    }
}

impl From<Error> for ::telegram_bot::Error {
    fn from(e: Error) -> Self {
        e.to_string().into()
    }
}
