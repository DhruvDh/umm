import os

def do():
    os.system("cargo clean")
    os.system("cargo build --release --target=x86_64-unknown-linux-musl")
    os.system("cp target/x86_64-unknown-linux-musl/release/umm ./umm")

if __name__ == "__main__":
    do()