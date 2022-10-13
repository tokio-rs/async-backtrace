test: miritest && loomtest

loomtest:
    RUSTFLAGS="--cfg loom" cargo test --release

miritest:
    cargo miri test