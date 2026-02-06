import { createRequire } from "node:module";
import type { NativeBinding } from "./types.js";

let native: NativeBinding | null = null;

export function loadNative(): NativeBinding {
  if (native) return native;

  const require = createRequire(import.meta.url);
  try {
    native = require("@vizejs/native") as NativeBinding;
    return native;
  } catch (e) {
    throw new Error(`Failed to load @vizejs/native. Make sure it's installed: ${String(e)}`);
  }
}
