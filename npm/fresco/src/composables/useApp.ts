/**
 * useApp - App context composable
 */

import { ref, provide, inject, type InjectionKey, type Ref } from "@vue/runtime-core";

const APP_KEY: InjectionKey<UseAppReturn> = Symbol("fresco-app");

export interface UseAppReturn {
  /** Terminal width */
  width: Ref<number>;
  /** Terminal height */
  height: Ref<number>;
  /** Whether app is running */
  isRunning: Ref<boolean>;
  /** Exit the app */
  exit: (code?: number) => void;
  /** Force re-render */
  render: () => void;
  /** Clear the screen */
  clear: () => void;
}

/**
 * Create app context (use at app root)
 */
export function createAppContext(): UseAppReturn {
  const width = ref(80);
  const height = ref(24);
  const isRunning = ref(true);

  // These would be connected to actual app instance
  const exit = (_code = 0) => {
    isRunning.value = false;
    // In real implementation, trigger app exit
  };

  const render = () => {
    // In real implementation, trigger re-render
  };

  const clear = () => {
    // In real implementation, clear screen
  };

  // Try to get terminal size
  if (typeof process !== "undefined" && process.stdout) {
    width.value = process.stdout.columns ?? 80;
    height.value = process.stdout.rows ?? 24;

    process.stdout.on?.("resize", () => {
      width.value = process.stdout.columns ?? 80;
      height.value = process.stdout.rows ?? 24;
    });
  }

  return {
    width,
    height,
    isRunning,
    exit,
    render,
    clear,
  };
}

/**
 * Provide app context to descendants
 */
export function provideApp(context: UseAppReturn) {
  provide(APP_KEY, context);
}

/**
 * Use app context
 */
export function useApp(): UseAppReturn {
  const context = inject(APP_KEY);

  if (!context) {
    // Return defaults if not in app context
    return {
      width: ref(80),
      height: ref(24),
      isRunning: ref(false),
      exit: () => {},
      render: () => {},
      clear: () => {},
    };
  }

  return context;
}

/**
 * Use terminal dimensions
 */
export function useTerminalSize() {
  const { width, height } = useApp();
  return { width, height };
}

/**
 * Exit handler
 */
export function useExit() {
  const { exit } = useApp();
  return exit;
}
