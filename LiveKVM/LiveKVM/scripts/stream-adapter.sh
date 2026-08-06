#!/usr/bin/env bash
set -euo pipefail

: "${KVMD_URL:=https://127.0.0.1}"
: "${KVMD_USER:?KVMD_USER is required}"
: "${KVMD_PASSWORD:?KVMD_PASSWORD is required}"
: "${LIVE777_WHIP_URL:=http://127.0.0.1:7777/whip/ipkvm}"
: "${RTP_PORT:=5002}"

runtime_dir=/run/ipkvm
sdp_file="$runtime_dir/input.sdp"
keepalive_pid=""
media_pid=""

cleanup() {
    [[ -n "$media_pid" ]] && kill "$media_pid" 2>/dev/null || true
    [[ -n "$keepalive_pid" ]] && kill "$keepalive_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM
install -d -m 0750 "$runtime_dir"
rm -f "$sdp_file"

auth_user="X-KVMD-User: ${KVMD_USER}"
auth_pass="X-KVMD-Passwd: ${KVMD_PASSWORD}"

# 保持stream=1会话，使无人打开官方页面时uStreamer仍持续编码。
websocat -k "${KVMD_URL}/api/ws?stream=1" -H "$auth_user" -H "$auth_pass" >/dev/null &
keepalive_pid=$!

# KVMD输出Annex-B H.264；仅重新封装为RTP，不进行二次编码。
websocat -b -B 10000000 -k "${KVMD_URL}/api/media/ws?video=h264" -H "$auth_user" -H "$auth_pass" |
    ffmpeg -hide_banner -loglevel warning -fflags nobuffer -flags low_delay \
        -f h264 -i pipe:0 -an -c:v copy -payload_type 96 \
        -f rtp -sdp_file "$sdp_file" "rtp://127.0.0.1:${RTP_PORT}?pkt_size=1200" &
media_pid=$!

for _ in {1..100}; do
    [[ -s "$sdp_file" ]] && break
    kill -0 "$media_pid" 2>/dev/null || { echo "H264/RTP process exited" >&2; exit 1; }
    sleep 0.1
done
[[ -s "$sdp_file" ]] || { echo "SDP was not created" >&2; exit 1; }

exec whipinto -i "$sdp_file" -w "$LIVE777_WHIP_URL"

