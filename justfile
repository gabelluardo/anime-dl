test:
    cargo test

test-ignored:
    cargo test -- --ignored

test-all:
    cargo test -- --include-ignored

pre-commit:
    cargo fmt --all
    cargo clippy -- -D warnings

commit m: test-all pre-commit
    git commit --no-verify -am "{{m}}"

amend: test-all pre-commit
    git commit --amend --no-verify