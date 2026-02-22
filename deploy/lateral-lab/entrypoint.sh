#!/usr/bin/env bash
set -euo pipefail

ROOT_PASSWORD="${ROOT_PASSWORD:-root}"
USERNAME="${USERNAME:-ctf}"
USER_PASSWORD="${USER_PASSWORD:-123456}"

echo "root:${ROOT_PASSWORD}" | chpasswd

if id "${USERNAME}" >/dev/null 2>&1; then
  echo "${USERNAME}:${USER_PASSWORD}" | chpasswd
else
  useradd -m -s /bin/bash "${USERNAME}"
  echo "${USERNAME}:${USER_PASSWORD}" | chpasswd
  usermod -aG sudo "${USERNAME}"
fi

mkdir -p /run/sshd
ssh-keygen -A >/dev/null 2>&1 || true

exec /usr/sbin/sshd -D -e
