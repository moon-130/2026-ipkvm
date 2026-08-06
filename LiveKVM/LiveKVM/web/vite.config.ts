import { defineConfig } from "vite";

export default defineConfig({
  base: "/ipkvm/",
  server: {
    proxy: {
      "/ipkvm/api": { target: "http://127.0.0.1:9080", rewrite: (path) => path.replace(/^\/ipkvm/, "") },
      "/ipkvm/ws": { target: "ws://127.0.0.1:9080", ws: true, rewrite: (path) => path.replace(/^\/ipkvm/, "") },
    },
  },
});
