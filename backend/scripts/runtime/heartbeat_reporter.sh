#!/bin/sh
set -eu

: "${HEARTBEAT_REPORT_URL:?HEARTBEAT_REPORT_URL is required}"
: "${HEARTBEAT_REPORT_TOKEN:?HEARTBEAT_REPORT_TOKEN is required}"

INTERVAL="${HEARTBEAT_INTERVAL_SECONDS:-30}"

while true; do
  curl -fsS -m 5 \
    -H "Content-Type: application/json" \
    -X POST "${HEARTBEAT_REPORT_URL}" \
    -d "{\"token\":\"${HEARTBEAT_REPORT_TOKEN}\"}" >/dev/null || true
  sleep "${INTERVAL}"
done
