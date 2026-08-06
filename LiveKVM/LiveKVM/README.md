# LiveKVM：Live777 + Rust 的局域网 IP-KVM 原型

本项目在 Geekworm KVM-A3/X630-A3 + Raspberry Pi 4B 的 PiKVM OS 上增加一套独立的视频和控制链路。它保留 PiKVM 的 TC358743 采集、uStreamer H.264 编码和 USB HID，实现以下团队自研部分：

- Rust/Axum 单控制者网关；
- Live777 WHIP/WHEP 视频分发集成；
- TypeScript Web 控制台；
- PiKVM H.264 → RTP → WHIP 适配器；
- systemd、Nginx、诊断、安装与回滚工具。

## 当前完成度

已完成可部署的软件原型和配置。尚未完成的部分必须在真实树莓派上执行：确认PiKVM版本、编译ARM64二进制、安装Live777、验证H.264接口以及进行延迟和稳定性测试。

## 目录

```text
src/                  Rust 网关
web/src/              TypeScript 前端源码
web/dist/             无需 Node.js 的可部署前端成品
config/               网关配置
deploy/               Live777、systemd、Nginx配置
scripts/              诊断、视频适配、安装、回滚脚本
docs/                  部署和测试说明
```

## 开发机检查

```bash
# Rust工具链可用时
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings

# 前端依赖可下载时
cd web
npm install
npm run check
npm run build

# 不依赖第三方包的静态检查
node --check web/dist/app.js
bash -n scripts/*.sh
```

## 树莓派部署摘要

1. 先导出完整TF卡镜像，并备份`/etc/kvmd`。
2. 执行`scripts/diagnose-pikvm.sh`，保存输出。
3. 准备ARM64版本的`ipkvm-gateway`、`live777`和`whipinto`。
4. 复制整个项目到树莓派，切换PiKVM文件系统为可写模式。
5. 运行`scripts/install-pikvm.sh`。
6. 修改`/etc/ipkvm/gateway.env`和`/etc/ipkvm/ipkvm.toml`。
7. 将`deploy/nginx-ipkvm.conf`合并到PiKVM Nginx HTTPS server块。
8. 启用三个服务，访问`https://<PiKVM-IP>/ipkvm/`。

完整步骤见[树莓派部署指南](docs/DEPLOYMENT.md)，实验方法见[验收与性能测试](docs/TESTING.md)。

## 控制协议

浏览器连接：

```text
WS /ws/control?client_id=<UUID>
```

示例消息：

```json
{"type":"key","seq":101,"payload":{"code":"KeyA","pressed":true}}
```

支持`key`、`mouse_move_abs`、`mouse_move_rel`、`mouse_button`、`wheel`、`ping`和`release_all`。绝对鼠标坐标由网页映射到KVMD要求的有符号范围`-32768..32767`。

## 重要限制

- 当前目标是局域网IPv4，不包含公网TURN。
- 网关会把浏览器的PiKVM登录Cookie回送到本机`/api/auth/check`验证；状态、控制WebSocket和WHEP视频代理都要求认证，未登录用户不能观看或控制。
- Live777与PiKVM原有WebRTC必须做实测对比，不能预设Live777延迟更低。
- `web/dist`是为无Node.js的PiKVM准备的成品；修改`web/src`后应重新运行Vite构建。
- 不允许livehal和uStreamer同时占用同一个采集设备。
