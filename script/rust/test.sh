#!/usr/bin/env bash
set -e

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

cargo test

bash tests/integration_test.sh
