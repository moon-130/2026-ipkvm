#!/usr/bin/env bash
set -u

section() { printf '\n### %s\n' "$1"; }
run() { printf '$ %s\n' "$*"; "$@" 2>&1 || true; }

section "OS and architecture"
run uname -a
run cat /etc/os-release
section "PiKVM packages"
run pacman -Q kvmd ustreamer
section "Video devices"
run v4l2-ctl --list-devices
run v4l2-ctl --all -d /dev/video0
section "Relevant services"
run systemctl --no-pager --full status kvmd kvmd-nginx kvmd-janus
section "H264 endpoint smoke test"
run curl -k -I https://127.0.0.1/api/hid
section "Resources"
run free -h
run vcgencmd measure_temp
run vcgencmd get_throttled

