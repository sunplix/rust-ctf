#!/usr/bin/env bash
set -euo pipefail

if ! command -v curl >/dev/null 2>&1; then
  echo "[single-image-smoke] curl is required" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "[single-image-smoke] jq is required" >&2
  exit 1
fi

api="${API_BASE:-http://127.0.0.1:8080/api/v1}"
admin_user="${ADMIN_USER:-admin}"
admin_pass="${ADMIN_PASSWORD:-admin123456}"
user_pass="${USER_PASSWORD:-password123}"
image_ref="${IMAGE_REF:-nginx:alpine}"
internal_port="${INTERNAL_PORT:-80}"
endpoint_protocol="${ENDPOINT_PROTOCOL:-http}"
max_retries="${MAX_RETRIES:-20}"
retry_sleep="${RETRY_SLEEP_SECONDS:-1}"

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

ts="$(date +%s)$(printf '%04x' "$RANDOM")"
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
    --arg t "Single Image Smoke $ts" \
    --arg s "single-image-smoke-$ts" \
    --arg st "$start_at" \
    --arg et "$end_at" \
    '{title:$t,slug:$s,visibility:"public",status:"running",start_at:$st,end_at:$et}')")"
contest_id="$(echo "$contest_resp" | jq -r '.id')"

challenge_resp="$(curl -fsS --max-time 20 -X POST "$api/admin/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn \
    --arg t "Single Image Challenge $ts" \
    --arg s "single-image-chal-$ts" \
    --arg img "$image_ref" \
    --argjson inport "$internal_port" \
    --arg protocol "$endpoint_protocol" \
    '{title:$t,slug:$s,category:"web",difficulty:"easy",challenge_type:"dynamic",flag_mode:"dynamic",status:"published",is_visible:true,metadata:{runtime:{mode:"single_image",image:$img,internal_port:$inport,protocol:$protocol,access_mode:"direct"}}}')")"
challenge_id="$(echo "$challenge_resp" | jq -r '.id')"

curl -fsS --max-time 20 -X POST "$api/admin/contests/$contest_id/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "{\"challenge_id\":\"$challenge_id\",\"sort_order\":1}" >/dev/null

username="si_smoke_${ts}"
register_resp="$(curl -fsS --max-time 20 -X POST "$api/auth/register" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg u "$username" --arg e "$username@example.com" --arg p "$user_pass" '{username:$u,email:$e,password:$p}')")"
user_token="$(echo "$register_resp" | jq -r '.access_token')"

curl -fsS --max-time 20 -X POST "$api/teams" \
  -H "Authorization: Bearer $user_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg n "si-smoke-team-$ts" '{name:$n}')" >/dev/null

start_resp="$(curl -fsS --max-time 120 -X POST "$api/instances/start" \
  -H "Authorization: Bearer $user_token" \
  -H 'Content-Type: application/json' \
  -d "{\"contest_id\":\"$contest_id\",\"challenge_id\":\"$challenge_id\"}")"

entrypoint="$(echo "$start_resp" | jq -r '.entrypoint_url')"
mode="$(echo "$start_resp" | jq -r '.network_access.mode // "direct"')"

case "$entrypoint" in
  "$endpoint_protocol"://*)
    ;;
  *)
    echo "[single-image-smoke] FAILED: unexpected entrypoint_url=$entrypoint" >&2
    echo "$start_resp" >&2
    exit 1
    ;;
esac

for ((i = 1; i <= max_retries; i++)); do
  if curl -fsS --max-time 3 "$entrypoint" >/dev/null 2>&1; then
    printf '[single-image-smoke] start ok\n'
    printf '  contest_id=%s\n' "$contest_id"
    printf '  challenge_id=%s\n' "$challenge_id"
    printf '  image=%s\n' "$image_ref"
    printf '  entrypoint=%s\n' "$entrypoint"
    printf '  mode=%s\n' "$mode"
    printf '  probe_try=%s\n' "$i"
    exit 0
  fi

  printf '[single-image-smoke] waiting entrypoint (try=%s)\n' "$i"
  sleep "$retry_sleep"
done

echo "[single-image-smoke] FAILED: endpoint probe timeout entrypoint=$entrypoint" >&2
exit 1
