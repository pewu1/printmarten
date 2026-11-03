#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
exec python3 scripts/build_local.py --platform linux --arch x64
