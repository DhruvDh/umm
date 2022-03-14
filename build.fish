#!/bin/fish

# cross build --target x86_64-unknown-linux-gnu --release
cargo build --target x86_64-unknown-linux-musl --release
cp target/x86_64-unknown-linux-musl/release/umm-check ./umm-check
cp target/x86_64-unknown-linux-musl/release/umm-run ./umm-run
cp target/x86_64-unknown-linux-musl/release/umm-clean ./umm-clean
cp target/x86_64-unknown-linux-musl/release/umm-test ./umm-test
cp target/x86_64-unknown-linux-musl/release/umm-info ./umm-info
cp target/x86_64-unknown-linux-musl/release/umm-doc-check ./umm-doc-check
cp target/x86_64-unknown-linux-musl/release/umm-grade ./umm-grade
