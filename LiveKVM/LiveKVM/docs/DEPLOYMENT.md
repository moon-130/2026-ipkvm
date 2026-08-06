# 树莓派部署指南

## 0. 不要跳过备份

关闭树莓派，取出TF卡，在开发电脑上制作整卡镜像。另行备份：

```bash
tar -czf kvmd-config-backup.tgz /etc/kvmd
```

重新插卡启动后，将本项目复制到树莓派，例如`/root/livekvm`。PiKVM OS默认只读，写入前执行`rw`，完成后执行`ro`。

## 1. 收集环境信息

```bash
cd /root/livekvm
./scripts/diagnose-pikvm.sh | tee diagnosis.txt
```

必须确认：

- 系统为ARM64；
- KVMD和uStreamer正常；
- 官方页面可以显示H.264视频；
- `/api/hid`可访问；
- 设备没有过热或欠压。

## 2. 准备程序

网关需要生成：

```text
target/aarch64-unknown-linux-gnu/release/ipkvm-gateway
```

可在ARM64 Linux开发环境运行`cargo build --release`后放入上述路径；也可以在Pi上临时安装Rust工具链完成编译。最终运行不需要Cargo。

从Live777官方Release选择ARM64 Linux版本，确认以下命令存在且可执行：

```bash
live777 --help
whipinto --help
ffmpeg -version
websocat --help
```

FFmpeg不需要WHIP muxer，因为本项目使用RTP加`whipinto`。

## 3. 创建KVMD专用账号

不要把管理员账号暴露给网页。使用PiKVM提供的用户管理工具创建`ipkvm-gateway`账号，并在本机验证：

```bash
curl -k -H 'X-KVMD-User:ipkvm-gateway' -H 'X-KVMD-Passwd:你的密码' https://127.0.0.1/api/hid
```

具体用户创建命令可能随KVMD版本变化，先运行`kvmd-htpasswd --help`，不得猜测参数后直接修改密码文件。

## 4. 安装文件

```bash
rw
cd /root/livekvm
./scripts/install-pikvm.sh
nano /etc/ipkvm/gateway.env
nano /etc/ipkvm/ipkvm.toml
chmod 600 /etc/ipkvm/gateway.env
```

将`deploy/nginx-ipkvm.conf`合并到现有HTTPS `server`块。不要覆盖PiKVM原Nginx配置。执行：

```bash
nginx -t
systemctl reload kvmd-nginx
```

## 5. 分阶段启动

先验证Live777：

```bash
systemctl start live777
curl -I http://127.0.0.1:7777/
journalctl -u live777 -f
```

再启动视频适配器：

```bash
systemctl start ipkvm-stream
journalctl -u ipkvm-stream -f
```

确认日志显示RTP与WHIP已建立后，启动网关：

```bash
systemctl start ipkvm-gateway
curl http://127.0.0.1:9080/api/status
```

最后访问`https://<PiKVM-IP>/ipkvm/`。全部通过后启用开机启动：

```bash
systemctl enable live777 ipkvm-stream ipkvm-gateway
ro
```

## 6. 故障与回滚

快速回滚不会删除日志和项目文件：

```bash
rw
cd /root/livekvm
./scripts/rollback-pikvm.sh
ro
```

常见检查：

- `401/403`：KVMD专用账号或密码错误；
- 页面存在但视频断开：检查`live777`与`ipkvm-stream`日志；
- FFmpeg没有产生SDP：检查原始H.264接口以及官方页面是否选择H.264模式；
- 鼠标方向或范围异常：确认前端发送`-32768..32767`；
- WHEP成功但无画面：检查浏览器开发者工具中的ICE candidate和局域网防火墙；
- `NO VIDEO SOURCE`：这是采集输入问题，Live777无法修复，应检查HDMI信号、分辨率、X630硬件复位模式和系统日志。

