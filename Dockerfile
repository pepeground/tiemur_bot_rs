FROM alpine:latest AS tiemur_build
RUN apk add --no-cache cargo build-base linux-headers
RUN USER=root cargo new tiemur_bot_rs
COPY Cargo.toml Cargo.lock /tiemur_bot_rs/
WORKDIR /tiemur_bot_rs
RUN cargo build --release
COPY src ./src
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache llvm-libunwind libstdc++
RUN adduser -D tiemur
USER tiemur
WORKDIR /home/tiemur/
RUN mkdir db
VOLUME /home/tiemur/db
COPY --from=tiemur_build /tiemur_bot_rs/target/release/tiemur_bot_rs ./app
ENV SLED_DB_DIR "/home/tiemur/db"
ENV TELEGRAM_TOKEN "token"
ENV DIST_RATIO 0.10
CMD ["./app"]
