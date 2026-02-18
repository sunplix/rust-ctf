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
user_pass="${USER_PASSWORD:-password123}"
lint_limit="${LINT_LIMIT:-300}"
fail_on_lint_errors="${FAIL_ON_LINT_ERRORS:-false}"
enable_wireguard_smoke="${ENABLE_WIREGUARD_SMOKE:-true}"
enable_single_image_smoke="${ENABLE_SINGLE_IMAGE_SMOKE:-true}"
enable_runtime_api_sanity="${ENABLE_RUNTIME_API_SANITY:-true}"
enable_scoreboard_ws_smoke="${ENABLE_SCOREBOARD_WS_SMOKE:-true}"

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

if [[ "$enable_wireguard_smoke" == "true" ]]; then
  step "wireguard smoke"
  API_BASE="$api" ADMIN_USER="$admin_user" ADMIN_PASSWORD="$admin_pass" \
    "$(dirname "$0")/wireguard_smoke.sh"
else
  step "wireguard smoke (skipped)"
fi

if [[ "$enable_single_image_smoke" == "true" ]]; then
  step "single image smoke"
  API_BASE="$api" ADMIN_USER="$admin_user" ADMIN_PASSWORD="$admin_pass" USER_PASSWORD="$user_pass" \
    "$(dirname "$0")/single_image_smoke.sh"
else
  step "single image smoke (skipped)"
fi

if [[ "$enable_runtime_api_sanity" == "true" ]]; then
  step "runtime metrics and reaper api sanity"
  instance_id="$(curl -fsS --max-time 30 "$api/admin/instances?status=running&limit=1" \
    -H "Authorization: Bearer $admin_token" | jq -r '.[0].id // empty')"
  if [[ -z "$instance_id" ]]; then
    echo "[runtime-regression] expected at least one running instance after smoke tests" >&2
    exit 1
  fi

  metrics_resp="$(curl -fsS --max-time 30 "$api/admin/instances/$instance_id/runtime-metrics" \
    -H "Authorization: Bearer $admin_token")"
  services_total="$(echo "$metrics_resp" | jq -r '.summary.services_total // 0')"
  running_services="$(echo "$metrics_resp" | jq -r '.summary.running_services // 0')"
  printf '  metrics instance=%s services_total=%s running_services=%s\n' "$instance_id" "$services_total" "$running_services"

  expired_reaper_resp="$(curl -fsS --max-time 30 -X POST "$api/admin/runtime/reaper/expired" \
    -H "Authorization: Bearer $admin_token")"
  stale_reaper_resp="$(curl -fsS --max-time 30 -X POST "$api/admin/runtime/reaper/stale" \
    -H "Authorization: Bearer $admin_token")"
  expired_mode="$(echo "$expired_reaper_resp" | jq -r '.mode // empty')"
  stale_mode="$(echo "$stale_reaper_resp" | jq -r '.mode // empty')"
  if [[ "$expired_mode" != "expired" || "$stale_mode" != "stale" ]]; then
    echo "[runtime-regression] unexpected reaper mode payload" >&2
    echo "$expired_reaper_resp" >&2
    echo "$stale_reaper_resp" >&2
    exit 1
  fi
  printf '  reaper expired(scanned=%s,reaped=%s) stale(scanned=%s,reaped=%s)\n' \
    "$(echo "$expired_reaper_resp" | jq -r '.scanned // 0')" \
    "$(echo "$expired_reaper_resp" | jq -r '.reaped // 0')" \
    "$(echo "$stale_reaper_resp" | jq -r '.scanned // 0')" \
    "$(echo "$stale_reaper_resp" | jq -r '.reaped // 0')"
else
  step "runtime metrics and reaper api sanity (skipped)"
fi

if [[ "$enable_scoreboard_ws_smoke" == "true" ]]; then
  step "scoreboard websocket smoke"
  if ! command -v node >/dev/null 2>&1; then
    echo "[runtime-regression] node is required for scoreboard websocket smoke" >&2
    exit 1
  fi
  API_BASE="$api" ADMIN_USER="$admin_user" ADMIN_PASSWORD="$admin_pass" USER_PASSWORD="$user_pass" \
    node "$(dirname "$0")/scoreboard_ws_smoke.mjs"
else
  step "scoreboard websocket smoke (skipped)"
fi

printf '\n[runtime-regression] PASSED\n'
