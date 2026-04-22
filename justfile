default:
    just dev

[parallel]
dev: dev-client dev-rust

dev-client:
    cd client && bun dev

dev-rust:
    cargo watch -x run -i client

build:
    just build-client
    just build-c
    just build-rust

build-client:
    cd client && bun run build

build-rust:
    cargo build --release

build-c:
  cmake -S tagger -B build
  cmake --build tagger/build

format:
    biome format --write
    cargo fmt

lint:
    cargo clippy
    biome lint --write
