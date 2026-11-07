#!/bin/bash
# Bundles web/app.js -> web/bundle.js. Uses ./node_modules when present; otherwise
# FOLIO_NODE_PATH can point at any directories that already contain marked,
# dompurify, highlight.js and esbuild (colon-separated).
set -euo pipefail
cd "$(dirname "$0")/.."
export NODE_PATH="$PWD/node_modules${FOLIO_NODE_PATH:+:$FOLIO_NODE_PATH}"
ESBUILD=""
IFS=: read -ra DIRS <<< "$NODE_PATH"
for d in "${DIRS[@]}"; do [ -x "$d/esbuild/bin/esbuild" ] && { ESBUILD="$d/esbuild/bin/esbuild"; break; }; done
[ -n "$ESBUILD" ] || { echo "esbuild not found; using prebundled web/bundle.js"; exit 0; }
"$ESBUILD" src-web/app.js --bundle --minify --target=safari16 --legal-comments=none --outfile=web/bundle.js
