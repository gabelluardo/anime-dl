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
    cargo nextest run --target x86_64-unknown-linux-musl

test-ignored:
    cargo nextest run --run-ignored=ignored-only --target x86_64-unknown-linux-musl

test-all:
    cargo nextest run --run-ignored=all --target x86_64-unknown-linux-musl

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