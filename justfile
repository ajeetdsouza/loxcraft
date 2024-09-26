default:
    @just --list

build:
    cargo build --release

build-all: build
    cd playground/ && just build

clean:
    cargo clean

clean-all: clean
    cd playground/ && just clean

fmt:
    cargo +nightly fmt --all

fmt-all: fmt
    cd playground/ && just fmt

lint:
    cargo +nightly fmt --all -- --check
    cargo clippy --all-features --all-targets --workspace -- --deny=warnings

lint-all: lint
    cd playground/ && just lint

run-playground:
    cd playground/ && just run

run-trace *args:
    cargo run --features='gc-stress,gc-trace,vm-trace' -- {{args}}

test *args:
    cargo nextest run --features='gc-stress,gc-trace,vm-trace' --workspace {{args}}

test-miri *args:
    MIRIFLAGS='-Zmiri-disable-isolation' cargo +nightly miri nextest run \
        --features='gc-stress,gc-trace,vm-trace' --no-default-features \
        --workspace {{args}}
