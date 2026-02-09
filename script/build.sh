#!/usr/bin/env bash
set -e

source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

cd "$(dirname "${BASH_SOURCE[0]}")/.."

${LOWKEY_DOCKER} run --rm \
    -v "$(pwd):/code" \
    -w /code \
    "${LOWKEY_IMAGE_DEV}" ./script/rust/compile.sh
