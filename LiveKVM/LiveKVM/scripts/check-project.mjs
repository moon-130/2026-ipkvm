import { readFileSync, existsSync } from "node:fs";

const required = [
  "Cargo.toml", "src/main.rs", "src/kvmd.rs", "web/dist/index.html", "web/dist/app.js",
  "deploy/live777.service", "deploy/ipkvm-stream.service", "deploy/ipkvm-gateway.service",
  "scripts/stream-adapter.sh", "scripts/rollback-pikvm.sh", "docs/DEPLOYMENT.md",
];
for (const path of required) {
  if (!existsSync(path)) throw new Error(`Missing required file: ${path}`);
}
const rust = readFileSync("src/main.rs", "utf8");
for (const route of ["/api/status", "/api/metrics", "/api/session/acquire", "/api/session/release", "/ws/control", "/media/{*path}"]) {
  if (!rust.includes(route)) throw new Error(`Missing gateway route: ${route}`);
}
const frontend = readFileSync("web/dist/app.js", "utf8");
for (const event of ["key", "mouse_move_abs", "mouse_move_rel", "mouse_button", "wheel", "release_all"]) {
  if (!frontend.includes(`'${event}'`)) throw new Error(`Missing frontend event: ${event}`);
}
if (!frontend.includes("65535-32768")) throw new Error("Absolute mouse mapping is not signed-centered");
const nginx = readFileSync("deploy/nginx-ipkvm.conf", "utf8");
if (!nginx.includes("location /ipkvm/")) throw new Error("Missing /ipkvm/ reverse proxy");
console.log("Project structure and protocol invariants: OK");
