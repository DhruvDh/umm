#!/bin/fish

cross build --target x86_64-unknown-linux-gnu --release
cp target/x86_64-unknown-linux-gnu/release/umm-check ./umm-check
cp target/x86_64-unknown-linux-gnu/release/umm-run ./umm-run
cp target/x86_64-unknown-linux-gnu/release/umm-clean ./umm-clean
cp target/x86_64-unknown-linux-gnu/release/umm-test ./umm-test

