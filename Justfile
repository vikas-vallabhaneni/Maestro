fmt:
    cargo fmt

check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

run *ARGS:
    cargo run -- {{ARGS}}
