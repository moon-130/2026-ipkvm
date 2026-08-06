#!/usr/bin/env bash
set -euo pipefail
[[ ${EUID:-$(id -u)} -eq 0 ]] || { echo "Run as root" >&2; exit 1; }
systemctl disable --now ipkvm-gateway.service ipkvm-stream.service live777.service 2>/dev/null || true
systemctl restart kvmd kvmd-nginx
echo "LiveKVM services disabled. Original PiKVM services restarted; files were retained for inspection."
