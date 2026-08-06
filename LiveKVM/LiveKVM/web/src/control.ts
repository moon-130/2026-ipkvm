type Payload = Record<string, string | number | boolean>;

export class ControlChannel {
  private ws?: WebSocket;
  private seq = 0;
  private pingSentAt = new Map<number, number>();
  private reconnectTimer?: number;
  private stopped = false;

  constructor(
    readonly clientId: string,
    private readonly onStatus: (connected: boolean, detail?: string) => void,
    private readonly onRtt: (rtt: number) => void,
  ) {}

  connect(): void {
    this.stopped = false;
    const protocol = location.protocol === "https:" ? "wss:" : "ws:";
    this.ws = new WebSocket(`${protocol}//${location.host}${appUrl("ws/control")}?client_id=${encodeURIComponent(this.clientId)}`);
    this.ws.onopen = () => this.onStatus(true);
    this.ws.onclose = () => {
      this.onStatus(false, "控制通道已断开");
      if (!this.stopped) this.reconnectTimer = window.setTimeout(() => this.connect(), 2000);
    };
    this.ws.onerror = () => this.onStatus(false, "控制通道错误");
    this.ws.onmessage = (event) => {
      const message = JSON.parse(String(event.data)) as { type: string; seq?: number; ok: boolean; message: string };
      if (message.message === "pong" && message.seq !== undefined) {
        const sent = this.pingSentAt.get(message.seq);
        if (sent !== undefined) {
          this.onRtt(Math.round(performance.now() - sent));
          this.pingSentAt.delete(message.seq);
        }
      }
    };
  }

  stop(): void {
    this.stopped = true;
    window.clearTimeout(this.reconnectTimer);
    this.ws?.close();
  }

  send(type: string, payload: Payload = {}): number {
    const seq = ++this.seq;
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type, seq, payload }));
    }
    return seq;
  }

  ping(): void {
    const seq = this.send("ping");
    this.pingSentAt.set(seq, performance.now());
  }

  async acquire(): Promise<boolean> {
    const response = await fetch(`${appUrl("api/session/acquire")}?client_id=${encodeURIComponent(this.clientId)}`, { method: "POST" });
    return response.ok;
  }

  async release(): Promise<void> {
    this.send("release_all");
    await fetch(`${appUrl("api/session/release")}?client_id=${encodeURIComponent(this.clientId)}`, { method: "POST" });
  }
}

function appUrl(path: string): string {
  const base = document.querySelector<HTMLMetaElement>('meta[name="app-base"]')?.content ?? "/";
  return `${base.replace(/\/$/, "")}/${path.replace(/^\//, "")}`;
}
