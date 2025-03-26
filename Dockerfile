FROM rust:1.85.1-slim-bookworm AS builder

WORKDIR /app

COPY Cargo.toml .
COPY Cargo.lock .


COPY src ./src

RUN cargo update
RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /app/target/release/number-generator .

EXPOSE 8080

CMD ["./number-generator"]
