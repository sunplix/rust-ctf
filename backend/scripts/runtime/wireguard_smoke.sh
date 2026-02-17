#!/usr/bin/env bash
set -euo pipefail

if ! command -v jq >/dev/null 2>&1; then
  echo "[wireguard-smoke] jq is required" >&2
  exit 1
fi

api="${API_BASE:-http://127.0.0.1:8080/api/v1}"
admin_user="${ADMIN_USER:-admin}"
admin_pass="${ADMIN_PASSWORD:-admin123456}"
user_pass="${USER_PASSWORD:-password123}"
max_retries="${MAX_RETRIES:-20}"
retry_sleep="${RETRY_SLEEP_SECONDS:-2}"

offset_minutes_utc() {
  local mins="$1"
  if date -u -v+1M +%Y-%m-%dT%H:%M:%SZ >/dev/null 2>&1; then
    if [[ "$mins" -ge 0 ]]; then
      date -u -v+"${mins}"M +%Y-%m-%dT%H:%M:%SZ
    else
      date -u -v"${mins}"M +%Y-%m-%dT%H:%M:%SZ
    fi
    return
  fi

  date -u -d "${mins} minutes" +%Y-%m-%dT%H:%M:%SZ
}

ts="$(date +%s)$(printf '%04x%04x' "$RANDOM" "$RANDOM")"
start_at="$(offset_minutes_utc -1)"
end_at="$(offset_minutes_utc 120)"

admin_login="$(curl -fsS --max-time 20 -X POST "$api/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"identifier\":\"$admin_user\",\"password\":\"$admin_pass\"}")"
admin_token="$(echo "$admin_login" | jq -r '.access_token')"

contest_resp="$(curl -fsS --max-time 20 -X POST "$api/admin/contests" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn \
    --arg t "WG Smoke $ts" \
    --arg s "wg-smoke-$ts" \
    --arg st "$start_at" \
    --arg et "$end_at" \
    '{title:$t,slug:$s,visibility:"public",status:"running",start_at:$st,end_at:$et}')")"
contest_id="$(echo "$contest_resp" | jq -r '.id')"

compose_template="$(cat <<'YAML'
services:
  web:
    image: alpine:3.20
    command: ["sh", "-c", "while true; do sleep 60; done"]
    networks:
      - "{{NETWORK_NAME}}"
networks:
  "{{NETWORK_NAME}}":
    driver: bridge
    ipam:
      config:
        - subnet: "{{SUBNET}}"
YAML
)"

challenge_resp="$(curl -fsS --max-time 20 -X POST "$api/admin/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn \
    --arg t "WG Smoke Challenge $ts" \
    --arg s "wg-smoke-chal-$ts" \
    --arg tmpl "$compose_template" \
    '{title:$t,slug:$s,category:"internal",difficulty:"normal",challenge_type:"internal",flag_mode:"dynamic",status:"published",is_visible:true,compose_template:$tmpl,metadata:{runtime:{mode:"compose",access_mode:"wireguard"}}}')")"
challenge_id="$(echo "$challenge_resp" | jq -r '.id')"

curl -fsS --max-time 20 -X POST "$api/admin/contests/$contest_id/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "{\"challenge_id\":\"$challenge_id\",\"sort_order\":1}" >/dev/null

username="wg_smoke_${ts}"
register_resp="$(curl -fsS --max-time 20 -X POST "$api/auth/register" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg u "$username" --arg e "$username@example.com" --arg p "$user_pass" '{username:$u,email:$e,password:$p}')")"
user_token="$(echo "$register_resp" | jq -r '.access_token')"

curl -fsS --max-time 20 -X POST "$api/teams" \
  -H "Authorization: Bearer $user_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg n "wg-smoke-team-$ts" '{name:$n}')" >/dev/null

start_resp="$(curl -fsS --max-time 120 -X POST "$api/instances/start" \
  -H "Authorization: Bearer $user_token" \
  -H 'Content-Type: application/json' \
  -d "{\"contest_id\":\"$contest_id\",\"challenge_id\":\"$challenge_id\"}")"

entrypoint="$(echo "$start_resp" | jq -r '.entrypoint_url')"
mode="$(echo "$start_resp" | jq -r '.network_access.mode')"
download_url="$(echo "$start_resp" | jq -r '.network_access.download_url')"

if [[ "$mode" != "wireguard" ]]; then
  echo "[wireguard-smoke] FAILED: expected mode=wireguard, got $mode" >&2
  echo "$start_resp" >&2
  exit 1
fi

printf '[wireguard-smoke] start ok\n'
printf '  contest_id=%s\n' "$contest_id"
printf '  challenge_id=%s\n' "$challenge_id"
printf '  entrypoint=%s\n' "$entrypoint"
printf '  mode=%s\n' "$mode"
printf '  download_url=%s\n' "$download_url"

for ((i = 1; i <= max_retries; i++)); do
  cfg_resp="$(curl -sS --max-time 20 "$api/instances/$contest_id/$challenge_id/wireguard-config" \
    -H "Authorization: Bearer $user_token" || true)"

  if echo "$cfg_resp" | jq -e '.content | contains("[Interface]") and contains("[Peer]")' >/dev/null 2>&1; then
    filename="$(echo "$cfg_resp" | jq -r '.filename')"
    head_line="$(echo "$cfg_resp" | jq -r '.content' | sed -n '1,6p' | tr '\n' '|')"
    printf '[wireguard-smoke] config ready (try=%s)\n' "$i"
    printf '  filename=%s\n' "$filename"
    printf '  head=%s\n' "$head_line"
    exit 0
  fi

  err_msg="$(echo "$cfg_resp" | jq -r '.error.message // "pending"' 2>/dev/null || echo 'pending')"
  printf '[wireguard-smoke] waiting (try=%s): %s\n' "$i" "$err_msg"
  sleep "$retry_sleep"
done

echo '[wireguard-smoke] FAILED: wireguard config not ready in time' >&2
exit 1
