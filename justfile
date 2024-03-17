pre-commit: test-all
    cargo fmt --all
    cargo clippy -- -D warnings

commit message: pre-commit
    git commit -am "{{message}}" 

fix: 
    cargo clippy --fix --allow-dirty --allow-staged

amend: pre-commit
    git commit --amend --no-verify

release:
    cargo build --release --locked --target x86_64-unknown-linux-musl

install:
    cargo install --path . --target x86_64-unknown-linux-musl

test:
    cargo nextest run

test-ignored:
    cargo nextest run --run-ignored=ignored-only

test-all:
    cargo nextest run --run-ignored=all

test-all-musl:
    cargo nextest run --run-ignored=all --target x86_64-unknown-linux-musl
