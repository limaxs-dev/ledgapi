#!/usr/bin/env bash
set -euo pipefail

mkdir -p /data
chown -R app:app /data
exec /usr/local/bin/ledgapi "$@"
