pre-commit: test-all
    cargo fmt --all
    cargo clippy -- -D warnings

commit message: pre-commit
    git commit -am "{{message}}" 

fix: 
    cargo clippy --fix --allow-dirty

amend: test-all pre-commit
    git commit --amend --no-verify

install:
    cargo install --path . --target x86_64-unknown-linux-musl

test:
    cargo nextest run

test-ignored:
    cargo nextest run --run-ignored=ignored-only

test-all:
    cargo nextest run --run-ignored=all

coverage:
    cargo tarpaulin \
        --skip-clean \
        --all-features \
        --ignored \
        --engine llvm \
        --exclude-files \
            src/main.rs \
            src/cli.rs \
            src/errors.rs \
            src/macros.rs