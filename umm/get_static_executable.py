import subprocess
import os

def do():
    env = os.environ
    subprocess.call([
        '''
        cargo clean
        cargo install --path=. && cp (which umm) ./umm
        exit
        '''
    ], shell=True, cwd=".", env=env)


if __name__ == '__main__':
    do()