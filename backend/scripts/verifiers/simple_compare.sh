#!/usr/bin/env bash
set -euo pipefail

expected="${1:-}"
submitted="${SUBMITTED_FLAG:-}"

if [[ -z "$expected" ]]; then
  echo "expected flag argument is required" >&2
  exit 2
fi

if [[ -z "$submitted" ]]; then
  echo "SUBMITTED_FLAG env is required" >&2
  exit 2
fi

if [[ "$submitted" == "$expected" ]]; then
  echo "script verifier accepted"
  exit 0
fi

echo "script verifier rejected"
exit 1
