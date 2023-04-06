pre-commit:
    cargo fmt --all
    cargo clippy -- -D warnings

commit m: test-all pre-commit
    git commit --no-verify -am "{{m}}"

amend: test-all pre-commit
    git commit --amend --no-verify

install:
    cargo install --path . --target x86_64-unknown-linux-musl

test:
    cargo test

test-ignored:
    cargo test -- --ignored

test-all:
    cargo test -- --include-ignored

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