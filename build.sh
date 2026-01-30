#!/bin/bash
set -e

cd "$(dirname "$0")"

pnpm run build:npx
(cd npx-cli && npm install)
