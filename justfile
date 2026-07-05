fix:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features -- -W warnings

release:
    cargo build --release --locked --target x86_64-unknown-linux-musl

install:
    cargo install --locked --path . --target x86_64-unknown-linux-musl 

test:
    cargo nextest run

test-ignored:
    cargo nextest run --run-ignored=ignored-only

test-all:
    cargo nextest run --run-ignored=all

test-all-musl:
    cargo nextest run --run-ignored=all --target x86_64-unknown-linux-musl

coverage:
    cargo llvm-cov nextest --open

coverage-lcov:
    cargo llvm-cov nextest --lcov --output-path coverage.lcov

coverage-all:
    cargo llvm-cov nextest --run-ignored all --open

coverage-all-lcov:
    cargo llvm-cov nextest --run-ignored all --lcov --output-path coverage.lcov

update-schema:
    rm schema/anilist_schema.json || true
    cargo build 
