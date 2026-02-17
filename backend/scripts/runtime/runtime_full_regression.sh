#!/usr/bin/env bash
set -euo pipefail

if ! command -v curl >/dev/null 2>&1; then
  echo "[runtime-regression] curl is required" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "[runtime-regression] jq is required" >&2
  exit 1
fi

api="${API_BASE:-http://127.0.0.1:8080/api/v1}"
admin_user="${ADMIN_USER:-admin}"
admin_pass="${ADMIN_PASSWORD:-admin123456}"
lint_limit="${LINT_LIMIT:-300}"
fail_on_lint_errors="${FAIL_ON_LINT_ERRORS:-false}"

step() {
  printf '\n[runtime-regression] %s\n' "$1"
}

step "health check"
health_resp="$(curl -fsS --max-time 10 "$api/health")"
health_status="$(echo "$health_resp" | jq -r '.status')"
if [[ "$health_status" != "ok" ]]; then
  echo "[runtime-regression] health check failed: $health_resp" >&2
  exit 1
fi
printf '  status=%s\n' "$health_status"

step "runtime template lint summary"
admin_login="$(curl -fsS --max-time 20 -X POST "$api/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"identifier\":\"$admin_user\",\"password\":\"$admin_pass\"}")"
admin_token="$(echo "$admin_login" | jq -r '.access_token')"

lint_url="$api/admin/challenges/runtime-template/lint?limit=$lint_limit&only_errors=false"
lint_resp="$(curl -fsS --max-time 30 "$lint_url" -H "Authorization: Bearer $admin_token")"
total_count="$(echo "$lint_resp" | jq -r '.scanned_total // 0')"
error_count="$(echo "$lint_resp" | jq -r '.error_count // 0')"
ok_count="$(echo "$lint_resp" | jq -r '.ok_count // 0')"
returned_total="$(echo "$lint_resp" | jq -r '.returned_total // 0')"
printf '  scanned=%s ok=%s error=%s returned=%s\n' "$total_count" "$ok_count" "$error_count" "$returned_total"

if [[ "$fail_on_lint_errors" == "true" ]] && [[ "$error_count" -gt 0 ]]; then
  echo "[runtime-regression] lint has errors and FAIL_ON_LINT_ERRORS=true" >&2
  echo "$lint_resp" | jq . >&2
  exit 1
fi

step "wireguard smoke"
API_BASE="$api" ADMIN_USER="$admin_user" ADMIN_PASSWORD="$admin_pass" \
  "$(dirname "$0")/wireguard_smoke.sh"

printf '\n[runtime-regression] PASSED\n'
