#!/bin/fish

cross build --target x86_64-unknown-linux-gnu --release
cp target/x86_64-unknown-linux-gnu/release/umm ./umm
