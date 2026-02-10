#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

echo "=== Installing Node dependencies ==="
pnpm i

echo ""
echo "=== Copying dev assets ==="
if [ ! -d dev_assets ] && [ -d dev_assets_seed ]; then
  cp -r dev_assets_seed dev_assets
  echo "Copied dev_assets_seed -> dev_assets"
else
  echo "dev_assets already exists, skipping."
fi

echo ""
echo "=== Building frontend ==="
cd frontend && pnpm run build && cd ..

echo ""
echo "Setup complete. Run ./dev.sh to start."
