import os

os.system("cargo clean")
os.system("cargo build --release --target=x86_64-unknown-linux-musl")
os.system("cp target/x86_64-unknown-linux-musl/release/umm ./umm")