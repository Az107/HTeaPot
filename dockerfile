FROM rust:latest AS builder


WORKDIR /usr/src/hteapot

COPY . .


RUN cargo build --release

FROM ubuntu:latest

RUN apt-get update && apt-get install -y  libssl-dev


WORKDIR /usr/bin


COPY --from=builder /usr/src/hteapot/target/release/hteapot .

WORKDIR /app

CMD ["/usr/bin/hteapot","config.toml"]
