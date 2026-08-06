#!/usr/bin/env bash
set -euo pipefail

[[ ${EUID:-$(id -u)} -eq 0 ]] || { echo "Run as root" >&2; exit 1; }
[[ -x target/aarch64-unknown-linux-gnu/release/ipkvm-gateway ]] || { echo "Missing aarch64 gateway binary" >&2; exit 1; }
[[ -d web/dist ]] || { echo "Missing web/dist; run npm run build" >&2; exit 1; }
for binary in live777 whipinto ffmpeg websocat; do
    command -v "$binary" >/dev/null || { echo "Missing dependency: $binary" >&2; exit 1; }
done

install -d -m 0755 /opt/ipkvm/bin /opt/ipkvm/web /etc/ipkvm
id ipkvm >/dev/null 2>&1 || useradd --system --home /opt/ipkvm --shell /usr/bin/nologin ipkvm
install -m 0755 target/aarch64-unknown-linux-gnu/release/ipkvm-gateway /opt/ipkvm/bin/
install -m 0755 "$(command -v live777)" /opt/ipkvm/bin/live777
install -m 0755 "$(command -v whipinto)" /opt/ipkvm/bin/whipinto
install -m 0755 scripts/stream-adapter.sh /opt/ipkvm/bin/
cp -R web/dist/. /opt/ipkvm/web/
install -m 0644 config/ipkvm.example.toml /etc/ipkvm/ipkvm.toml
install -m 0644 deploy/live777.toml /etc/ipkvm/live777.toml
[[ -f /etc/ipkvm/gateway.env ]] || install -m 0600 deploy/ipkvm.env.example /etc/ipkvm/gateway.env
install -m 0644 deploy/ipkvm-gateway.service deploy/live777.service deploy/ipkvm-stream.service /etc/systemd/system/
systemctl daemon-reload
echo "Installed. Edit /etc/ipkvm/gateway.env and ipkvm.toml, add nginx config, then enable services."
