set windows-shell := ["powershell", "-NoProfile", "-Command"]
set shell := ["sh", "-c"]

default:
    just dev

dev: build-tagger dev-run

[parallel]
dev-run: dev-client dev-rust

dev-client:
    bun --cwd client dev

dev-rust:
    cargo watch -x run -i client -i src/state/config/themes -i justfile

build:
    just build-client
    just build-rust

build-client:
    bun --cwd client build

build-rust: build-tagger
    cargo bundle --release

[working-directory('crates/tagger-compiler')]
build-tagger:
    cargo run -- tags.tree -o ../../target/output/tagger.bin

test:
    cargo test -- --no-capture

biome := if os_family() == "windows" { ".\\node_modules\\.bin\\biome.exe" } else { "./node_modules/.bin/biome" }

format:
    {{ biome }} format --write
    cargo fmt

lint:
    cargo clippy
    {{ biome }} lint --write
