#!/usr/bin/env bash
set -euo pipefail

echo "=== Tunnel Cloudflare *.siphion.dev ==="
echo ""
echo "  Espone tutto ciò che Caddy serve su :443"
echo ""
echo "Premi Ctrl+C per chiudere l'accesso pubblico."
echo ""

cloudflared tunnel run siphion
