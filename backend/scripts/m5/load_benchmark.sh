#!/usr/bin/env bash
set -euo pipefail

if ! command -v curl >/dev/null 2>&1; then
  echo "[m5-load] curl is required" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "[m5-load] jq is required" >&2
  exit 1
fi

api="${API_BASE:-http://127.0.0.1:8080/api/v1}"
admin_user="${ADMIN_USER:-admin}"
admin_pass="${ADMIN_PASSWORD:-admin123456}"
user_pass="${USER_PASSWORD:-password123}"
team_count="${TEAM_COUNT:-20}"
requests_total="${REQUESTS_TOTAL:-400}"
concurrency="${CONCURRENCY:-20}"
valid_flag_percent="${VALID_FLAG_PERCENT:-35}"
warmup_rounds="${WARMUP_ROUNDS:-1}"
output_dir="${OUTPUT_DIR:-/tmp/rust-ctf-m5-load-$(date +%Y%m%d-%H%M%S)}"

if [[ "$team_count" -lt 1 || "$team_count" -gt 300 ]]; then
  echo "[m5-load] TEAM_COUNT must be in 1..300" >&2
  exit 1
fi
if [[ "$requests_total" -lt 1 || "$requests_total" -gt 20000 ]]; then
  echo "[m5-load] REQUESTS_TOTAL must be in 1..20000" >&2
  exit 1
fi
if [[ "$concurrency" -lt 1 || "$concurrency" -gt 500 ]]; then
  echo "[m5-load] CONCURRENCY must be in 1..500" >&2
  exit 1
fi
if [[ "$valid_flag_percent" -lt 0 || "$valid_flag_percent" -gt 100 ]]; then
  echo "[m5-load] VALID_FLAG_PERCENT must be in 0..100" >&2
  exit 1
fi
if [[ "$warmup_rounds" -lt 0 || "$warmup_rounds" -gt 20 ]]; then
  echo "[m5-load] WARMUP_ROUNDS must be in 0..20" >&2
  exit 1
fi

mkdir -p "$output_dir"
tokens_file="$output_dir/player_tokens.txt"
results_tsv="$output_dir/submission_results.tsv"
summary_json="$output_dir/summary.json"

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
  printf '\n[m5-load] %s\n' "$1"
}

calc_rps() {
  local req="$1"
  local sec="$2"
  if [[ "$sec" -le 0 ]]; then
    sec=1
  fi
  awk "BEGIN { printf \"%.2f\", $req / $sec }"
}

ts_tag="$(date +%s)$(printf '%04x' "$RANDOM")"
valid_flag="ctf{m5-load-${ts_tag}}"
start_at="$(offset_minutes_utc -1)"
end_at="$(offset_minutes_utc 120)"

step "login admin"
admin_login="$(curl -fsS --max-time 20 -X POST "$api/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{\"identifier\":\"$admin_user\",\"password\":\"$admin_pass\"}")"
admin_token="$(echo "$admin_login" | jq -r '.access_token')"

step "prepare contest/challenge"
contest_resp="$(curl -fsS --max-time 20 -X POST "$api/admin/contests" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn \
    --arg t "M5 Load Contest $ts_tag" \
    --arg s "m5-load-contest-$ts_tag" \
    --arg st "$start_at" \
    --arg et "$end_at" \
    '{title:$t,slug:$s,visibility:"public",status:"running",start_at:$st,end_at:$et}')")"
contest_id="$(echo "$contest_resp" | jq -r '.id')"

challenge_resp="$(curl -fsS --max-time 20 -X POST "$api/admin/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "$(jq -cn \
    --arg t "M5 Load Challenge $ts_tag" \
    --arg s "m5-load-challenge-$ts_tag" \
    --arg f "$valid_flag" \
    '{title:$t,slug:$s,category:"web",difficulty:"easy",challenge_type:"static",flag_mode:"static",flag_hash:$f,status:"published",is_visible:true,static_score:100,min_score:100,max_score:100}')")"
challenge_id="$(echo "$challenge_resp" | jq -r '.id')"

curl -fsS --max-time 20 -X POST "$api/admin/contests/$contest_id/challenges" \
  -H "Authorization: Bearer $admin_token" \
  -H 'Content-Type: application/json' \
  -d "{\"challenge_id\":\"$challenge_id\",\"sort_order\":1}" >/dev/null

step "register players and create teams (count=$team_count)"
rm -f "$tokens_file"
for i in $(seq 1 "$team_count"); do
  username="$(printf 'm5u%s%03d' "$ts_tag" "$i")"
  team_name="$(printf 'm5t%s%03d' "$ts_tag" "$i")"
  register_resp="$(curl -fsS --max-time 20 -X POST "$api/auth/register" \
    -H 'Content-Type: application/json' \
    -d "$(jq -cn --arg u "$username" --arg e "$username@example.com" --arg p "$user_pass" '{username:$u,email:$e,password:$p}')")"
  token="$(echo "$register_resp" | jq -r '.access_token')"
  curl -fsS --max-time 20 -X POST "$api/teams" \
    -H "Authorization: Bearer $token" \
    -H 'Content-Type: application/json' \
    -d "$(jq -cn --arg n "$team_name" '{name:$n}')" >/dev/null
  echo "$token" >>"$tokens_file"
done

sample_player_token="$(sed -n '1p' "$tokens_file")"

if [[ "$warmup_rounds" -gt 0 ]]; then
  step "warmup submissions (rounds=$warmup_rounds)"
  for _ in $(seq 1 "$warmup_rounds"); do
    while IFS= read -r token; do
      curl -sS --max-time 10 -X POST "$api/submissions" \
        -H "Authorization: Bearer $token" \
        -H 'Content-Type: application/json' \
        -d "{\"contest_id\":\"$contest_id\",\"challenge_id\":\"$challenge_id\",\"flag\":\"$valid_flag\"}" >/dev/null
    done <"$tokens_file"
  done
fi

step "run benchmark (requests=$requests_total concurrency=$concurrency)"
rm -f "$results_tsv"
start_epoch="$(date +%s)"

export API_BASE="$api"
export CONTEST_ID="$contest_id"
export CHALLENGE_ID="$challenge_id"
export TOKENS_FILE="$tokens_file"
export TEAM_COUNT="$team_count"
export VALID_FLAG="$valid_flag"
export VALID_FLAG_PERCENT="$valid_flag_percent"

seq 1 "$requests_total" | xargs -P "$concurrency" -I{} bash -c '
  i="$1"
  token_line=$(( ((i - 1) % TEAM_COUNT) + 1 ))
  token="$(sed -n "${token_line}p" "$TOKENS_FILE")"

  if (( ((i * 17 + token_line * 31) % 100) < VALID_FLAG_PERCENT )); then
    flag="$VALID_FLAG"
  else
    flag="ctf{wrong-${i}}"
  fi

  body_file="$(mktemp)"
  http_meta="$(curl -sS --max-time 15 -o "$body_file" -w "%{http_code}\t%{time_total}" \
    -X POST "$API_BASE/submissions" \
    -H "Authorization: Bearer $token" \
    -H "Content-Type: application/json" \
    -d "{\"contest_id\":\"$CONTEST_ID\",\"challenge_id\":\"$CHALLENGE_ID\",\"flag\":\"$flag\"}")"
  http_code="${http_meta%%$'\''\t'\''*}"
  time_total="${http_meta#*$'\''\t'\''}"
  verdict="$(jq -r ".verdict // .error.code // \"unknown\"" "$body_file" 2>/dev/null || echo "unknown")"
  rm -f "$body_file"
  printf "%s\t%s\t%s\t%s\n" "$i" "$http_code" "$time_total" "$verdict"
' _ {} >"$results_tsv"

end_epoch="$(date +%s)"
duration_seconds="$((end_epoch - start_epoch))"
rps="$(calc_rps "$requests_total" "$duration_seconds")"

step "calculate metrics"
total_rows="$(wc -l <"$results_tsv" | tr -d ' ')"
http_2xx="$(awk -F'\t' '$2 >= 200 && $2 < 300 { count++ } END { print count + 0 }' "$results_tsv")"
accepted_total="$(awk -F'\t' '$4 == "accepted" { count++ } END { print count + 0 }' "$results_tsv")"
rate_limited_total="$(awk -F'\t' '$4 == "rate_limited" { count++ } END { print count + 0 }' "$results_tsv")"
wrong_total="$(awk -F'\t' '$4 == "wrong_answer" { count++ } END { print count + 0 }' "$results_tsv")"
error_total="$(awk -F'\t' '$2 >= 500 || $4 == "internal_error" { count++ } END { print count + 0 }' "$results_tsv")"

times_sorted="$output_dir/response_times.sorted"
awk -F'\t' '{ print $3 }' "$results_tsv" | sort -n >"$times_sorted"
times_count="$(wc -l <"$times_sorted" | tr -d ' ')"
p50_index="$(( (times_count + 1) / 2 ))"
p95_index="$(( (times_count * 95 + 99) / 100 ))"
p99_index="$(( (times_count * 99 + 99) / 100 ))"
p50_seconds="$(sed -n "${p50_index}p" "$times_sorted")"
p95_seconds="$(sed -n "${p95_index}p" "$times_sorted")"
p99_seconds="$(sed -n "${p99_index}p" "$times_sorted")"
avg_seconds="$(awk '{ sum += $1 } END { if (NR == 0) { print 0 } else { printf "%.6f", sum / NR } }' "$times_sorted")"

scoreboard_entries="$(curl -fsS --max-time 20 "$api/contests/$contest_id/scoreboard" \
  -H "Authorization: Bearer $sample_player_token" | jq 'length')"

jq -n \
  --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg contest_id "$contest_id" \
  --arg challenge_id "$challenge_id" \
  --arg output_dir "$output_dir" \
  --argjson team_count "$team_count" \
  --argjson requests_total "$requests_total" \
  --argjson concurrency "$concurrency" \
  --argjson valid_flag_percent "$valid_flag_percent" \
  --argjson duration_seconds "$duration_seconds" \
  --argjson rps "$rps" \
  --argjson total_rows "$total_rows" \
  --argjson http_2xx "$http_2xx" \
  --argjson accepted_total "$accepted_total" \
  --argjson rate_limited_total "$rate_limited_total" \
  --argjson wrong_total "$wrong_total" \
  --argjson error_total "$error_total" \
  --argjson avg_seconds "$avg_seconds" \
  --argjson p50_seconds "${p50_seconds:-0}" \
  --argjson p95_seconds "${p95_seconds:-0}" \
  --argjson p99_seconds "${p99_seconds:-0}" \
  --argjson scoreboard_entries "$scoreboard_entries" \
  '{
    generated_at: $generated_at,
    contest_id: $contest_id,
    challenge_id: $challenge_id,
    output_dir: $output_dir,
    params: {
      team_count: $team_count,
      requests_total: $requests_total,
      concurrency: $concurrency,
      valid_flag_percent: $valid_flag_percent
    },
    metrics: {
      duration_seconds: $duration_seconds,
      requests_per_second: $rps,
      total_rows: $total_rows,
      http_2xx: $http_2xx,
      accepted_total: $accepted_total,
      rate_limited_total: $rate_limited_total,
      wrong_total: $wrong_total,
      error_total: $error_total,
      scoreboard_entries: $scoreboard_entries,
      response_time_seconds: {
        avg: $avg_seconds,
        p50: $p50_seconds,
        p95: $p95_seconds,
        p99: $p99_seconds
      }
    },
    artifacts: {
      tokens_file: ($output_dir + "/player_tokens.txt"),
      results_tsv: ($output_dir + "/submission_results.tsv"),
      summary_json: ($output_dir + "/summary.json")
    }
  }' >"$summary_json"

cat "$summary_json"
printf '\n[m5-load] summary saved: %s\n' "$summary_json"
