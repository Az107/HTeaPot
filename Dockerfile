FROM rust AS builder

WORKDIR /app
COPY Cargo.lock Cargo.lock
COPY Cargo.toml Cargo.toml
COPY src ./src

RUN cargo build --release

FROM ubuntu

COPY --from=builder /app/target/release/hteapot /bin/hteapot

EXPOSE 80

WORKDIR /config

ENTRYPOINT ["/bin/hteapot"]
CMD ["config.toml"]
