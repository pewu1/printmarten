"""Build and validate a native local distribution without system installation."""
import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def run(*args, env=None):
    subprocess.run([str(a) for a in args], cwd=ROOT, env=env, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--platform', required=True, choices=['macos', 'windows', 'linux'])
    parser.add_argument('--arch', required=True, choices=['x64', 'arm64'])
    args = parser.parse_args()
    host = {'Darwin': 'macos', 'Windows': 'windows', 'Linux': 'linux'}.get(platform.system())
    arch = {'x86_64': 'x64', 'AMD64': 'x64', 'arm64': 'arm64', 'aarch64': 'arm64'}.get(platform.machine())
    if (host, arch) != (args.platform, args.arch):
        parser.error('Run this script on the matching operating system and architecture.')
    required = ['cargo', 'rustup'] + {'macos': ['clang', 'hdiutil', 'codesign'], 'windows': ['cl', 'makensis'], 'linux': ['pkg-config', 'dpkg-deb']}[host]
    for tool in required:
        if not shutil.which(tool):
            parser.error(f'Missing prerequisite: {tool}; see docs/development.md')
    venv = ROOT / 'target/local-tools'
    run(sys.executable, '-m', 'venv', venv)
    python = venv / ('Scripts/python.exe' if host == 'windows' else 'bin/python')
    run(python, '-m', 'pip', 'install', '--upgrade', 'pip')
    run(python, '-m', 'pip', 'install', '-r', ROOT / 'scripts/requirements.txt')
    run(python, ROOT / 'scripts/build_native.py')
    environment = os.environ.copy()
    environment['PRINTMARTEN_NATIVE_LIB_DIR'] = str(ROOT / 'target/native' / ('bin' if host == 'windows' else 'lib'))
    if host == 'macos':
        environment['MACOSX_DEPLOYMENT_TARGET'] = '13.0'
    run('cargo', 'build', '--release', '--locked', env=environment)
    run('cargo', 'install', 'cargo-about', '--version', '0.8.2', '--locked')
    run('cargo', 'about', 'generate', '--locked', '--fail', 'packaging/licenses.hbs', '--output-file', 'target/THIRD_PARTY_LICENSES.html')
    run(python, 'scripts/package.py', '--platform', host, '--arch', arch, '--sha', 'local')
    run(python, 'scripts/validate_packages.py', '--platform', host, '--arch', arch, '--sha', 'local')
    print(f'Validated packages: {ROOT / "dist"}')


if __name__ == '__main__':
    main()
