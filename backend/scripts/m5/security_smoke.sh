#!/usr/bin/env bash
set -euo pipefail

if ! command -v curl >/dev/null 2>&1; then
  echo "[m5-security] curl is required" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "[m5-security] jq is required" >&2
  exit 1
fi

api="${API_BASE:-http://127.0.0.1:8080/api/v1}"
admin_user="${ADMIN_USER:-admin}"
admin_pass="${ADMIN_PASSWORD:-admin123456}"
user_pass="${USER_PASSWORD:-password123}"
output_dir="${OUTPUT_DIR:-/tmp/rust-ctf-m5-security-$(date +%Y%m%d-%H%M%S)}"

mkdir -p "$output_dir"
results_tsv="$output_dir/security_results.tsv"

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

step() {
  printf '\n[m5-security] %s\n' "$1"
}

run_case() {
  local case_name="$1"
  local expected_code="$2"
  local method="$3"
  local url="$4"
  local token="${5:-}"
  local body="${6:-}"
  local expect_message="${7:-}"

  local body_file
  body_file="$(mktemp)"

  local -a curl_args=(
    -sS --max-time 20 -o "$body_file" -w "%{http_code}" -X "$method" "$url"
  )
  if [[ -n "$token" ]]; then
    curl_args+=(-H "Authorization: Bearer $token")
  fi
  if [[ -n "$body" ]]; then
    curl_args+=(-H "Content-Type: application/json" -d "$body")
  fi

  local http_code
  http_code="$(curl "${curl_args[@]}")"

  local ok="1"
  local reason="ok"
  if [[ "$http_code" != "$expected_code" ]]; then
    ok="0"
    reason="unexpected_http_code"
  elif [[ -n "$expect_message" ]]; then
    local msg
    msg="$(jq -r '.error.message // empty' "$body_file" 2>/dev/null || true)"
    if [[ "$msg" != *"$expect_message"* ]]; then
      ok="0"
      reason="unexpected_error_message"
    fi
  fi

  printf '%s\t%s\t%s\t%s\t%s\n' "$case_name" "$ok" "$http_code" "$expected_code" "$reason" >>"$results_tsv"

  if [[ "$ok" != "1" ]]; then
    echo "[m5-security] case failed: $case_name (http=$http_code expected=$expected_code reason=$reason)" >&2
    echo "[m5-security] response body:" >&2
    cat "$body_file" >&2
  fi

  rm -f "$body_file"
}

rm -f "$results_tsv"
printf 'case\tok\thttp_code\texpected\treason\n' >"$results_tsv"

ts_tag="$(date +%s)$(printf '%04x' "$RANDOM")"
start_at="$(offset_minutes_utc -1)"
end_at="$(offset_minutes_utc 120)"

step "login admin"
admin_login="$(curl -fsS --max-time 20 -X POST "$api/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"identifier\":\"$admin_user\",\"password\":\"$admin_pass\"}")"
admin_token="$(echo "$admin_login" | jq -r '.access_token')"

step "prepare users"
user_a="m5secA${ts_tag}"
user_b="m5secB${ts_tag}"
user_a_token="$(curl -fsS --max-time 20 -X POST "$api/auth/register" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg u "$user_a" --arg e "$user_a@example.com" --arg p "$user_pass" '{username:$u,email:$e,password:$p}')" \
  | jq -r '.access_token')"
user_b_token="$(curl -fsS --max-time 20 -X POST "$api/auth/register" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg u "$user_b" --arg e "$user_b@example.com" --arg p "$user_pass" '{username:$u,email:$e,password:$p}')" \
  | jq -r '.access_token')"

curl -fsS --max-time 20 -X POST "$api/teams" \
  -H "Authorization: Bearer $user_a_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn --arg n "m5secTeam${ts_tag}" '{name:$n}')" >/dev/null

step "prepare contests/challenges"
running_contest_id="$(curl -fsS --max-time 20 -X POST "$api/admin/contests" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn \
    --arg t "M5 Security Running $ts_tag" \
    --arg s "m5-sec-running-$ts_tag" \
    --arg st "$start_at" \
    --arg et "$end_at" \
    '{title:$t,slug:$s,visibility:"public",status:"running",start_at:$st,end_at:$et}')" \
  | jq -r '.id')"

draft_contest_id="$(curl -fsS --max-time 20 -X POST "$api/admin/contests" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn \
    --arg t "M5 Security Draft $ts_tag" \
    --arg s "m5-sec-draft-$ts_tag" \
    --arg st "$start_at" \
    --arg et "$end_at" \
    '{title:$t,slug:$s,visibility:"public",status:"draft",start_at:$st,end_at:$et}')" \
  | jq -r '.id')"

static_flag="ctf{m5-security-${ts_tag}}"
static_challenge_id="$(curl -fsS --max-time 20 -X POST "$api/admin/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn \
    --arg t "M5 Security Static $ts_tag" \
    --arg s "m5-sec-static-$ts_tag" \
    --arg f "$static_flag" \
    '{title:$t,slug:$s,category:"web",difficulty:"easy",challenge_type:"static",flag_mode:"static",flag_hash:$f,status:"published",is_visible:true,static_score:100,min_score:100,max_score:100}')" \
  | jq -r '.id')"

curl -fsS --max-time 20 -X POST "$api/admin/contests/$running_contest_id/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "{\"challenge_id\":\"$static_challenge_id\",\"sort_order\":1}" >/dev/null

curl -fsS --max-time 20 -X POST "$api/admin/contests/$draft_contest_id/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "{\"challenge_id\":\"$static_challenge_id\",\"sort_order\":1}" >/dev/null

step "run security cases"
run_case \
  "unauth_me_rejected" \
  "401" \
  "GET" \
  "$api/auth/me"

run_case \
  "player_admin_endpoint_forbidden" \
  "403" \
  "GET" \
  "$api/admin/users" \
  "$user_a_token"

run_case \
  "no_team_submission_forbidden" \
  "403" \
  "POST" \
  "$api/submissions" \
  "$user_b_token" \
  "{\"contest_id\":\"$running_contest_id\",\"challenge_id\":\"$static_challenge_id\",\"flag\":\"$static_flag\"}" \
  "permission denied"

run_case \
  "draft_contest_submission_rejected" \
  "400" \
  "POST" \
  "$api/submissions" \
  "$user_a_token" \
  "{\"contest_id\":\"$draft_contest_id\",\"challenge_id\":\"$static_challenge_id\",\"flag\":\"$static_flag\"}" \
  "contest is not running"

run_case \
  "static_challenge_instance_start_rejected" \
  "400" \
  "POST" \
  "$api/instances/start" \
  "$user_a_token" \
  "{\"contest_id\":\"$running_contest_id\",\"challenge_id\":\"$static_challenge_id\"}" \
  "challenge type does not require runtime instance"

run_case \
  "scoreboard_requires_auth" \
  "401" \
  "GET" \
  "$api/contests/$running_contest_id/scoreboard"

pass_count="$(awk -F'\t' 'NR > 1 && $2 == "1" { count++ } END { print count + 0 }' "$results_tsv")"
fail_count="$(awk -F'\t' 'NR > 1 && $2 != "1" { count++ } END { print count + 0 }' "$results_tsv")"
total_count="$(awk -F'\t' 'NR > 1 { count++ } END { print count + 0 }' "$results_tsv")"

summary_json="$output_dir/summary.json"
jq -n \
  --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg output_dir "$output_dir" \
  --argjson total "$total_count" \
  --argjson passed "$pass_count" \
  --argjson failed "$fail_count" \
  --arg results_tsv "$results_tsv" \
  '{
    generated_at: $generated_at,
    totals: { total: $total, passed: $passed, failed: $failed },
    artifacts: {
      results_tsv: $results_tsv,
      output_dir: $output_dir
    }
  }' >"$summary_json"

cat "$summary_json"
printf '\n[m5-security] detail: %s\n' "$results_tsv"

if [[ "$fail_count" -gt 0 ]]; then
  exit 1
fi
