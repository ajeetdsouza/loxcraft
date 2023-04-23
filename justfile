profile *args:
	cargo run --features='pprof' --no-default-features --profile='pprof' -- {{args}}

test *args:
    MIRIFLAGS='-Zmiri-disable-isolation' cargo +nightly miri nextest run \
        --all --features='gc-stress,gc-trace,vm-trace' --no-default-features \
        {{args}}
