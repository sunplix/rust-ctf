#!/usr/bin/env bash
set -euo pipefail

api_base="${API_BASE_URL:-http://127.0.0.1:8080/api/v1}"
identifier="${ADMIN_IDENTIFIER:-admin}"
password="${ADMIN_PASSWORD:-admin123456}"
limit="${LINT_LIMIT:-500}"
only_errors="${ONLY_ERRORS:-false}"
challenge_type="${CHALLENGE_TYPE:-}"
status="${CHALLENGE_STATUS:-}"
keyword="${KEYWORD:-}"

login_resp=$(curl -sS -X POST "$api_base/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"identifier\":\"$identifier\",\"password\":\"$password\"}")

token=$(echo "$login_resp" | sed -n 's/.*"access_token":"\([^"]*\)".*/\1/p')
if [[ -z "$token" ]]; then
  echo "failed to login admin user: $identifier" >&2
  echo "$login_resp" >&2
  exit 1
fi

query="limit=$limit&only_errors=$only_errors"
if [[ -n "$challenge_type" ]]; then
  query+="&challenge_type=$challenge_type"
fi
if [[ -n "$status" ]]; then
  query+="&status=$status"
fi
if [[ -n "$keyword" ]]; then
  query+="&keyword=$keyword"
fi

url="$api_base/admin/challenges/runtime-template/lint?$query"
resp=$(curl -sS "$url" -H "Authorization: Bearer $token")

if command -v jq >/dev/null 2>&1; then
  echo "$resp" | jq .
else
  echo "$resp"
fi
