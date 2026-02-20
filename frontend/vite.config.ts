import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

const allowedHosts = (process.env.VITE_ALLOWED_HOSTS ?? "")
  .split(",")
  .map((host) => host.trim())
  .filter(Boolean);

export default defineConfig({
  plugins: [vue()],
  server: {
    host: "0.0.0.0",
    port: 5173,
    // For reverse-proxy deployments that still use Vite dev server.
    allowedHosts: allowedHosts.length > 0 ? allowedHosts : undefined
  }
});
