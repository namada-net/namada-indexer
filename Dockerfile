FROM lukemathwalker/cargo-chef:latest-rust-1.85-bookworm AS chef
RUN apt-get update && apt-get install -y protobuf-compiler build-essential clang-tools-14

FROM chef AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ARG PACKAGE
RUN cargo build --release --locked --bin ${PACKAGE}

FROM debian:bookworm AS runtime
RUN apt-get update && apt-get install -y libpq5 ca-certificates curl
WORKDIR /app
ARG PACKAGE
COPY --from=builder /app/target/release/${PACKAGE} ./
RUN mv ./${PACKAGE} ./service

