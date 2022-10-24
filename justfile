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

    ver_taskdump=$(msrv taskdump)
    ver_taskdump_attributes=$(msrv taskdump-attributes)

    if [[ "$ver_taskdump" == "$ver_taskdump_attributes" ]]; then
        echo "Same MSRV ($ver_taskdump) found in 'taskdump' and 'taskdump-attributes'." | tee -a $GITHUB_STEP_SUMMARY
        exit 0
    else
        echo "Different MSRVs found in 'taskdump' ($ver_taskdump) and '$taskdump-attributes' ($ver_taskdump_attributes)." \
            | tee -a $GITHUB_STEP_SUMMARY >&2
        exit 1
    fi
