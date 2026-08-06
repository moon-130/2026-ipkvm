export type WhepState = "idle" | "connecting" | "playing" | "reconnecting" | "failed";

export class WhepPlayer {
  private pc?: RTCPeerConnection;
  private resourceUrl?: string;
  private reconnectTimer?: number;
  private stopped = false;

  constructor(
    private readonly video: HTMLVideoElement,
    private readonly endpoint: string,
    private readonly onState: (state: WhepState, detail?: string) => void,
  ) {}

  async start(): Promise<void> {
    this.stopped = false;
    await this.connect(false);
  }

  async restart(): Promise<void> {
    await this.cleanup();
    this.stopped = false;
    await this.connect(true);
  }

  async stop(): Promise<void> {
    this.stopped = true;
    window.clearTimeout(this.reconnectTimer);
    await this.cleanup();
    this.onState("idle");
  }

  private async connect(reconnecting: boolean): Promise<void> {
    this.onState(reconnecting ? "reconnecting" : "connecting");
    const pc = new RTCPeerConnection({ bundlePolicy: "max-bundle" });
    this.pc = pc;
    pc.addTransceiver("video", { direction: "recvonly" });
    pc.ontrack = (event) => {
      this.video.srcObject = event.streams[0] ?? new MediaStream([event.track]);
      void this.video.play();
      this.onState("playing");
    };
    pc.onconnectionstatechange = () => {
      if (["failed", "disconnected"].includes(pc.connectionState) && !this.stopped) {
        this.scheduleReconnect(`WebRTC ${pc.connectionState}`);
      }
    };
    try {
      const offer = await pc.createOffer();
      await pc.setLocalDescription(offer);
      await waitForIceGathering(pc, 1500);
      const response = await fetch(this.endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/sdp" },
        body: pc.localDescription?.sdp,
      });
      if (!response.ok) throw new Error(`WHEP returned ${response.status}`);
      const location = response.headers.get("Location");
      if (location) this.resourceUrl = new URL(location, window.location.href).toString();
      await pc.setRemoteDescription({ type: "answer", sdp: await response.text() });
    } catch (error) {
      await this.cleanup();
      this.scheduleReconnect(String(error));
    }
  }

  private scheduleReconnect(detail: string): void {
    if (this.stopped || this.reconnectTimer) return;
    this.onState("failed", detail);
    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = undefined;
      void this.restart();
    }, 3000);
  }

  private async cleanup(): Promise<void> {
    if (this.resourceUrl) {
      void fetch(this.resourceUrl, { method: "DELETE" }).catch(() => undefined);
      this.resourceUrl = undefined;
    }
    this.pc?.close();
    this.pc = undefined;
    this.video.srcObject = null;
  }
}

function waitForIceGathering(pc: RTCPeerConnection, timeoutMs: number): Promise<void> {
  if (pc.iceGatheringState === "complete") return Promise.resolve();
  return new Promise((resolve) => {
    const timer = window.setTimeout(resolve, timeoutMs);
    const listener = () => {
      if (pc.iceGatheringState === "complete") {
        window.clearTimeout(timer);
        pc.removeEventListener("icegatheringstatechange", listener);
        resolve();
      }
    };
    pc.addEventListener("icegatheringstatechange", listener);
  });
}

