FROM lukemathwalker/cargo-chef:latest-rust-1.96-bookworm AS chef
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --manifest-path relay/Cargo.toml --bin ech-board-relay

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ffmpeg ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/ech-board-relay /usr/local/bin/ech-board-relay
ENTRYPOINT ["/usr/local/bin/ech-board-relay"]
