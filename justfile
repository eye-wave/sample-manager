set windows-shell := ["powershell", "-NoProfile", "-Command"]
set shell := ["sh", "-c"]

default:
    just dev

[parallel]
dev: dev-client dev-rust

dev-client:
    bun --cwd client dev

dev-rust:
    cargo watch -x run -i client -i src/state/config/themes -i justfile

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

    cmake -S tagger -B tagger/build
    cmake --build tagger/build --config Release

test: build-c
    cargo test -- --no-capture

biome := if os_family() == "windows" { ".\\node_modules\\.bin\\biome.exe" } else { "./node_modules/.bin/biome" }

format:
    {{ biome }} format --write
    cargo fmt

lint:
    cargo clippy
    {{ biome }} lint --write
