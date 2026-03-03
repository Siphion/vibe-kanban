#!/usr/bin/env bash
set -euo pipefail

echo "=== Esposizione pubblica su internet ==="
echo ""
echo "  https://vk.siphion.dev    (Vibe Kanban)"
echo "  https://code.siphion.dev  (code-server)"
echo ""
echo "Premi Ctrl+C per chiudere l'accesso pubblico."
echo ""

cloudflared tunnel run siphion
