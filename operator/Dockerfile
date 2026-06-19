FROM lukemathwalker/cargo-chef:latest-rust-1.96-bookworm AS chef
WORKDIR /src

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS build
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --manifest-path operator/Cargo.toml --bin ech-board-operator

FROM gcr.io/distroless/cc-debian12
COPY --from=build /src/target/release/ech-board-operator /usr/local/bin/ech-board-operator
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/ech-board-operator"]
