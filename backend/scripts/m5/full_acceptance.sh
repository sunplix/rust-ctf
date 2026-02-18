#!/usr/bin/env bash
set -euo pipefail

api="${API_BASE:-http://127.0.0.1:8080/api/v1}"
admin_user="${ADMIN_USER:-admin}"
admin_pass="${ADMIN_PASSWORD:-admin123456}"
user_pass="${USER_PASSWORD:-password123}"
output_dir="${OUTPUT_DIR:-/tmp/rust-ctf-m5-acceptance-$(date +%Y%m%d-%H%M%S)}"

load_team_count="${LOAD_TEAM_COUNT:-20}"
load_requests_total="${LOAD_REQUESTS_TOTAL:-400}"
load_concurrency="${LOAD_CONCURRENCY:-20}"
load_valid_flag_percent="${LOAD_VALID_FLAG_PERCENT:-35}"
load_warmup_rounds="${LOAD_WARMUP_ROUNDS:-1}"

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd "$script_dir/../../.." && pwd)"
backend_dir="$project_root/backend"
runtime_regression_script="$backend_dir/scripts/runtime/runtime_full_regression.sh"
security_smoke_script="$backend_dir/scripts/m5/security_smoke.sh"
load_benchmark_script="$backend_dir/scripts/m5/load_benchmark.sh"

mkdir -p "$output_dir"
runtime_log="$output_dir/runtime_full_regression.log"
security_log="$output_dir/security_smoke.log"
load_log="$output_dir/load_benchmark.log"
report_md="$output_dir/M5_ACCEPTANCE_REPORT.md"
summary_json="$output_dir/summary.json"

step() {
  printf '\n[m5-acceptance] %s\n' "$1"
}

runtime_status="passed"
security_status="passed"
load_status="passed"

step "run runtime full regression"
set +e
API_BASE="$api" ADMIN_USER="$admin_user" ADMIN_PASSWORD="$admin_pass" USER_PASSWORD="$user_pass" \
  "$runtime_regression_script" \
  >"$runtime_log" 2>&1
runtime_code="$?"
set -e
if [[ "$runtime_code" -ne 0 ]]; then
  runtime_status="failed"
fi

step "run security smoke"
set +e
security_json="$(
  API_BASE="$api" ADMIN_USER="$admin_user" ADMIN_PASSWORD="$admin_pass" USER_PASSWORD="$user_pass" \
    OUTPUT_DIR="$output_dir/security" \
    "$security_smoke_script" 2>&1
)"
security_code="$?"
set -e
printf '%s\n' "$security_json" >"$security_log"
if [[ "$security_code" -ne 0 ]]; then
  security_status="failed"
fi

step "run load benchmark"
set +e
load_json="$(
  API_BASE="$api" ADMIN_USER="$admin_user" ADMIN_PASSWORD="$admin_pass" USER_PASSWORD="$user_pass" \
    TEAM_COUNT="$load_team_count" REQUESTS_TOTAL="$load_requests_total" CONCURRENCY="$load_concurrency" \
    VALID_FLAG_PERCENT="$load_valid_flag_percent" WARMUP_ROUNDS="$load_warmup_rounds" \
    OUTPUT_DIR="$output_dir/load" \
    "$load_benchmark_script" 2>&1
)"
load_code="$?"
set -e
printf '%s\n' "$load_json" >"$load_log"
if [[ "$load_code" -ne 0 ]]; then
  load_status="failed"
fi

runtime_tail="$(tail -n 20 "$runtime_log" | sed 's/[[:cntrl:]]//g')"
security_summary="$(echo "$security_json" | sed -n '/^{/,/^}/p' | head -n 80)"
load_summary="$(echo "$load_json" | sed -n '/^{/,/^}/p' | head -n 200)"

load_p95="n/a"
load_rps="n/a"
load_errors="n/a"
if [[ "$load_status" == "passed" ]]; then
  load_p95="$(echo "$load_summary" | jq -r '.metrics.response_time_seconds.p95 // "n/a"' 2>/dev/null || echo "n/a")"
  load_rps="$(echo "$load_summary" | jq -r '.metrics.requests_per_second // "n/a"' 2>/dev/null || echo "n/a")"
  load_errors="$(echo "$load_summary" | jq -r '.metrics.error_total // "n/a"' 2>/dev/null || echo "n/a")"
fi

cat >"$report_md" <<EOF
# M5 Acceptance Report

- Generated At (UTC): $(date -u +%Y-%m-%dT%H:%M:%SZ)
- API Base: $api

## Step Status

- Runtime Full Regression: $runtime_status
- Security Smoke: $security_status
- Load Benchmark: $load_status

## Load Snapshot

- p95 latency (seconds): $load_p95
- throughput (req/s): $load_rps
- server/internal errors: $load_errors

## Artifacts

- runtime log: $runtime_log
- security log: $security_log
- load log: $load_log
- this report: $report_md

## Runtime Tail

\`\`\`
$runtime_tail
\`\`\`

## Security Summary

\`\`\`json
$security_summary
\`\`\`

## Load Summary

\`\`\`json
$load_summary
\`\`\`
EOF

jq -n \
  --arg generated_at "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg api "$api" \
  --arg runtime_status "$runtime_status" \
  --arg security_status "$security_status" \
  --arg load_status "$load_status" \
  --arg report_md "$report_md" \
  --arg runtime_log "$runtime_log" \
  --arg security_log "$security_log" \
  --arg load_log "$load_log" \
  '{
    generated_at: $generated_at,
    api_base: $api,
    statuses: {
      runtime_full_regression: $runtime_status,
      security_smoke: $security_status,
      load_benchmark: $load_status
    },
    artifacts: {
      report_markdown: $report_md,
      runtime_log: $runtime_log,
      security_log: $security_log,
      load_log: $load_log
    }
  }' >"$summary_json"

cat "$summary_json"
printf '\n[m5-acceptance] report: %s\n' "$report_md"

if [[ "$runtime_status" != "passed" || "$security_status" != "passed" || "$load_status" != "passed" ]]; then
  exit 1
fi
