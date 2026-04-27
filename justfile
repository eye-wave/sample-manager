set windows-shell := ["powershell", "-NoProfile", "-Command"]
set shell := ["sh", "-c"]

default:
    just dev

[parallel]
dev: dev-client dev-rust

dev-client:
    bun --cwd client dev

dev-rust:
    cargo watch -x run -i client

build:
    just build-client
    just build-c
    just build-rust

build-client:
    bun --cwd client build

build-rust:
    cargo build --release

build-c:
    cargo run -p compiler

    cmake -S tagger -B tagger/build -DCMAKE_BUILD_TYPE=Release
    cmake --build tagger/build

test: build-c
    cargo test -- --no-capture

format:
    biome format --write
    cargo fmt

lint:
    cargo clippy
    biome lint --write
