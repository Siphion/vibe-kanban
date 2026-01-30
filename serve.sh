#!/bin/bash
set -e

cd "$(dirname "$0")"

exec node npx-cli/bin/cli.js "$@"
