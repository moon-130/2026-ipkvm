import "./style.css";
import { ControlChannel } from "./control";
import { WhepPlayer, WhepState } from "./whep";

type Status = {
  kvmd: boolean; hid: boolean; live777: boolean; controller?: string;
  viewers: number; width: number; height: number; fps: number; whep_url: string;
};

const app = document.querySelector<HTMLDivElement>("#app")!;
const appUrl = (path: string): string => `${(document.querySelector<HTMLMetaElement>('meta[name="app-base"]')?.content ?? "/").replace(/\/$/, "")}/${path.replace(/^\//, "")}`;
app.innerHTML = `
  <header><div><p class="eyebrow">自主研发 · 局域网 IP-KVM</p><h1>LiveKVM 控制台</h1></div><div id="clock"></div></header>
  <main>
    <section class="viewer-card">
      <div class="viewer-toolbar">
        <div class="status-row">
          <span id="video-status" class="pill warn">视频连接中</span>
          <span id="hid-status" class="pill">HID检测中</span>
          <span id="control-status" class="pill">只读</span>
        </div>
        <div class="toolbar-actions">
          <select id="mouse-mode" aria-label="鼠标模式"><option value="absolute">绝对鼠标</option><option value="relative">相对鼠标</option></select>
          <button id="reconnect">重连视频</button><button id="fullscreen">全屏</button>
        </div>
      </div>
      <div id="stage" class="stage" tabindex="0">
        <video id="video" autoplay muted playsinline></video>
        <div id="overlay" class="overlay"><strong>点击画面申请控制权</strong><span>按 Esc 退出控制</span></div>
      </div>
    </section>
    <aside>
      <section class="panel"><h2>连接状态</h2><dl><div><dt>分辨率</dt><dd id="resolution">—</dd></div><div><dt>控制RTT</dt><dd id="rtt">—</dd></div><div><dt>在线浏览器</dt><dd id="viewers">—</dd></div><div><dt>CPU</dt><dd id="cpu">—</dd></div><div><dt>内存</dt><dd id="memory">—</dd></div></dl></section>
      <section class="panel"><h2>控制操作</h2><button id="acquire" class="primary">申请控制权</button><button id="release">释放控制权</button><button id="emergency" class="danger">紧急释放全部按键</button><p id="hint">当前为只读观看模式。</p></section>
      <section class="panel compact"><h2>操作提示</h2><p>点击画面捕获键鼠；按 Esc 或切换窗口自动释放。BIOS中鼠标异常时切换为相对模式。</p></section>
    </aside>
  </main>`;

const el = <T extends HTMLElement>(id: string) => document.querySelector<T>(`#${id}`)!;
const video = el<HTMLVideoElement>("video");
const stage = el<HTMLDivElement>("stage");
const overlay = el<HTMLDivElement>("overlay");
const clientId = crypto.randomUUID();
let hasControl = false;
let whep: WhepPlayer | undefined;
let latestMove: { x: number; y: number } | undefined;
let moveFrame = 0;
const pressedKeys = new Set<string>();

const control = new ControlChannel(clientId, (connected, detail) => {
  setPill("hid-status", connected, connected ? "控制通道在线" : (detail ?? "控制通道离线"));
}, (rtt) => { el("rtt").textContent = `${rtt} ms`; });
control.connect();

function setPill(id: string, ok: boolean, text: string): void {
  const target = el(id);
  target.textContent = text;
  target.className = `pill ${ok ? "ok" : "warn"}`;
}

function videoState(state: WhepState, detail?: string): void {
  const labels: Record<WhepState, string> = { idle: "视频已停止", connecting: "视频连接中", playing: "视频正常", reconnecting: "视频重连中", failed: "视频暂不可用" };
  setPill("video-status", state === "playing", labels[state]);
  el("video-status").title = detail ?? "";
}

async function refreshStatus(): Promise<void> {
  try {
    const status = await fetch(appUrl("api/status")).then((r) => r.json()) as Status;
    setPill("hid-status", status.hid, status.hid ? "HID正常" : "HID离线");
    el("resolution").textContent = `${status.width}×${status.height} / ${status.fps}fps`;
    el("viewers").textContent = String(status.viewers);
    if (!whep) { whep = new WhepPlayer(video, status.whep_url, videoState); void whep.start(); }
  } catch { setPill("hid-status", false, "请先登录PiKVM"); el("hint").textContent = "请先在PiKVM主页登录，再打开本页面。"; }
}

async function refreshMetrics(): Promise<void> {
  try {
    const m = await fetch(appUrl("api/metrics")).then((r) => r.json()) as { cpu_percent: number; memory_used_bytes: number; memory_total_bytes: number };
    el("cpu").textContent = `${m.cpu_percent.toFixed(1)}%`;
    el("memory").textContent = `${(m.memory_used_bytes / 1048576).toFixed(0)} / ${(m.memory_total_bytes / 1048576).toFixed(0)} MB`;
  } catch { /* 状态轮询会继续重试 */ }
}

async function acquire(): Promise<void> {
  hasControl = await control.acquire();
  overlay.classList.toggle("hidden", hasControl);
  setPill("control-status", hasControl, hasControl ? "控制中" : "控制权被占用");
  el("hint").textContent = hasControl ? "键鼠事件正在发送到被控主机。" : "另一位用户正在控制。";
  if (hasControl) stage.focus();
}

async function release(): Promise<void> {
  if (hasControl) await control.release();
  hasControl = false;
  pressedKeys.clear();
  overlay.classList.remove("hidden");
  setPill("control-status", false, "只读");
  el("hint").textContent = "当前为只读观看模式。";
  if (document.pointerLockElement) document.exitPointerLock();
}

stage.addEventListener("click", () => { if (!hasControl) void acquire(); });
el("acquire").addEventListener("click", () => void acquire());
el("release").addEventListener("click", () => void release());
el("emergency").addEventListener("click", () => { control.send("release_all"); void release(); });
el("reconnect").addEventListener("click", () => void whep?.restart());
el("fullscreen").addEventListener("click", () => void stage.requestFullscreen());

window.addEventListener("keydown", (event) => {
  if (!hasControl) return;
  if (event.code === "Escape") { void release(); return; }
  event.preventDefault();
  if (!pressedKeys.has(event.code)) {
    pressedKeys.add(event.code);
    control.send("key", { code: event.code, pressed: true });
  }
});
window.addEventListener("keyup", (event) => {
  if (!hasControl) return;
  event.preventDefault();
  pressedKeys.delete(event.code);
  control.send("key", { code: event.code, pressed: false });
});
window.addEventListener("blur", () => void release());

stage.addEventListener("contextmenu", (event) => event.preventDefault());
stage.addEventListener("mousedown", (event) => {
  if (!hasControl) return;
  event.preventDefault();
  control.send("mouse_button", { button: mouseButton(event.button), pressed: true });
});
stage.addEventListener("mouseup", (event) => {
  if (!hasControl) return;
  control.send("mouse_button", { button: mouseButton(event.button), pressed: false });
});
stage.addEventListener("wheel", (event) => {
  if (!hasControl) return;
  event.preventDefault();
  control.send("wheel", { dy: Math.max(-127, Math.min(127, Math.round(-event.deltaY / 4))) });
}, { passive: false });
stage.addEventListener("mousemove", (event) => {
  if (!hasControl) return;
  const mode = el<HTMLSelectElement>("mouse-mode").value;
  if (mode === "relative") {
    control.send("mouse_move_rel", { dx: clamp16(event.movementX), dy: clamp16(event.movementY) });
    return;
  }
  const rect = video.getBoundingClientRect();
  latestMove = {
    x: Math.round(Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width)) * 65535 - 32768),
    y: Math.round(Math.max(0, Math.min(1, (event.clientY - rect.top) / rect.height)) * 65535 - 32768),
  };
  if (!moveFrame) moveFrame = requestAnimationFrame(flushMove);
});

function flushMove(): void {
  moveFrame = 0;
  if (latestMove) { control.send("mouse_move_abs", latestMove); latestMove = undefined; }
}
function mouseButton(button: number): string { return (["left", "middle", "right"] as const)[button] ?? "left"; }
function clamp16(value: number): number { return Math.max(-32768, Math.min(32767, value)); }

setInterval(() => control.ping(), 2000);
setInterval(() => void refreshStatus(), 3000);
setInterval(() => void refreshMetrics(), 3000);
setInterval(() => { el("clock").textContent = new Date().toLocaleTimeString("zh-CN", { hour12: false }); }, 1000);
void refreshStatus();
void refreshMetrics();
