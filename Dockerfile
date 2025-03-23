FROM rust:1.65 as builder

WORKDIR /app

COPY Cargo.toml .
COPY src ./src

RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /app

COPY --from=builder /app/target/release/number-generator .

EXPOSE 3030

CMD ["./number-generator"]
