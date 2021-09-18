import subprocess
import os

def do():
    env = os.environ
    subprocess.call([
        '''
        cargo clean
        cargo build --release
        cp target/release/umm ./umm
        cargo clean
        exit
        '''
    ], shell=True, cwd=".", env=env)


if __name__ == '__main__':
    do()