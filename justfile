test: miri && loom

loom:
    RUSTFLAGS="--cfg loom" cargo test --release --tests

miri:
    cargo miri test

readme:
    just generate-readme > README.md

generate-readme:
    cargo readme -r backtrace -t ../README.tpl --no-indent-headings

check-msrv:
    #!/usr/bin/env bash
    set -e

    # Usage: msrv <crate-name>
    function msrv {
        cargo metadata --format-version 1 | jq -r ".packages[] | select(.name == \"$1\").rust_version"
    }

    ver_async_backtrace=$(msrv async-backtrace)
    ver_async_backtrace_macros=$(msrv async-backtrace-attributes)

    if [[ "$ver_async_backtrace" == "$ver_async_backtrace_macros" ]]; then
        echo "Same MSRV ($ver_async_backtrace) found in 'async-backtrace' and 'async-backtrace-attributes'." | tee -a $GITHUB_STEP_SUMMARY
        exit 0
    else
        echo "Different MSRVs found in 'async-backtrace' ($ver_async_backtrace) and 'async-backtrace-attributes' ($ver_async_backtrace_macros)." \
            | tee -a $GITHUB_STEP_SUMMARY >&2
        exit 1
    fi
