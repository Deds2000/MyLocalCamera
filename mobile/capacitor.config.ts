import type { CapacitorConfig } from "@capacitor/cli";

const config: CapacitorConfig = {
  appId: "com.localcamera.app",
  appName: "LocalCamera",
  webDir: "dist",
  // CapConfig's default `debug` logs Capacitor bridge methodData, which would
  // include pairing tokens passed to LocalCamera.connect. Native app logs
  // remain explicit and secret-free.
  loggingBehavior: "none",
};

export default config;
