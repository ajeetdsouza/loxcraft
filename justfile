build:
    cargo build --release
    cd playground/ && npm run build

clean:
    cargo clean
    cd playground/ && npm run clean

fmt:
    cargo +nightly fmt
    cd playground/ && npm run fmt

lint:
    cargo clippy --all -- --deny=warnings
    cd playground/ && npm run lint

profile *args:
	cargo run --features='pprof' --no-default-features --profile='pprof' -- {{args}}

test *args:
    MIRIFLAGS='-Zmiri-disable-isolation' cargo +nightly miri nextest run \
        --all --features='gc-stress,gc-trace,vm-trace' --no-default-features \
        {{args}}
    cargo nextest run --all --features='gc-stress,gc-trace,vm-trace' {{args}}
