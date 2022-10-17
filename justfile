test: miritest && loomtest

loomtest:
    RUSTFLAGS="--cfg loom" cargo test --release --tests

miritest:
    cargo miri test