ARG SUI_TOOLS_IMAGE

FROM lukemathwalker/cargo-chef:latest-rust-1.96-bookworm AS chef
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build
ARG RUST_TARGET
RUN apt-get update && apt-get install -y musl-tools && rm -rf /var/lib/apt/lists/*
RUN rustup target add ${RUST_TARGET}
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --target ${RUST_TARGET}
COPY . .
RUN cargo build --release --target ${RUST_TARGET} --manifest-path worker/Cargo.toml --bin ech-board-worker

FROM ${SUI_TOOLS_IMAGE}
ARG RUST_TARGET
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/${RUST_TARGET}/release/ech-board-worker /usr/local/bin/ech-board-worker
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/ech-board-worker"]
