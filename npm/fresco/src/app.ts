/**
 * Fresco App - Application instance management
 */

import { type Component, type App as VueApp, ref, type Ref } from "@vue/runtime-core";
import { createRenderer, treeToRenderNodes, type FrescoElement } from "./renderer.js";

// Event types
export interface KeyEvent {
  type: "key";
  key?: string;
  char?: string;
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
}

export interface ResizeEvent {
  type: "resize";
  width: number;
  height: number;
}

export type InputEvent = KeyEvent | ResizeEvent;

// Global event state
export const lastKeyEvent: Ref<KeyEvent | null> = ref(null);

// Import native bindings
let native: typeof import("@vizejs/fresco-native") | null = null;

async function loadNative() {
  if (!native) {
    native = await import("@vizejs/fresco-native");
  }
  return native;
}

/**
 * App options
 */
export interface AppOptions {
  /** Enable mouse support */
  mouse?: boolean;
  /** Exit on Ctrl+C */
  exitOnCtrlC?: boolean;
  /** Custom error handler */
  onError?: (error: Error) => void;
  /** Debug mode - logs render tree */
  debug?: boolean;
}

/**
 * Fresco App instance
 */
export interface App {
  /** Mount the app */
  mount(): Promise<void>;
  /** Unmount the app */
  unmount(): Promise<void>;
  /** Wait for exit */
  waitUntilExit(): Promise<void>;
  /** Render the app */
  render(): void;
  /** Get terminal info */
  getTerminalInfo(): Promise<{ width: number; height: number }>;
}

/**
 * Create a Fresco TUI app
 */
export function createApp(rootComponent: Component, options: AppOptions = {}): App {
  const { mouse = false, exitOnCtrlC = true, onError } = options;

  let vueApp: VueApp | null = null;
  let rootElement: FrescoElement | null = null;
  let mounted = false;
  let running = false;
  let exitResolve: (() => void) | null = null;
  let needsRender = true;

  const { createApp: createVueApp } = createRenderer();

  async function mount() {
    if (mounted) return;

    const n = await loadNative();

    // Initialize terminal
    if (mouse) {
      n.initTerminalWithMouse();
    } else {
      n.initTerminal();
    }

    // Initialize layout engine
    n.initLayout();

    // Create Vue app with custom renderer
    const app = createVueApp(rootComponent);

    // Create a root element for mounting
    rootElement = {
      id: -1,
      type: "root",
      props: {
        style: {
          width: "100%",
          height: "100%",
          flexDirection: "column",
          justifyContent: "flex-start",
          alignItems: "flex-start",
          alignContent: "flex-start",
        },
      },
      children: [],
      parent: null,
    };

    app.mount(rootElement);
    vueApp = app;

    mounted = true;
    running = true;
    needsRender = true;

    // Start event loop
    void eventLoop();
  }

  async function unmount() {
    if (!mounted) return;

    running = false;

    const n = await loadNative();
    n.restoreTerminal();

    if (vueApp) {
      vueApp.unmount();
      vueApp = null;
    }

    rootElement = null;
    mounted = false;

    if (exitResolve) {
      exitResolve();
    }
  }

  async function waitUntilExit(): Promise<void> {
    return new Promise((resolve) => {
      exitResolve = resolve;
    });
  }

  function render() {
    if (!native || !mounted || !rootElement) {
      return;
    }

    try {
      // Convert Vue tree to render nodes
      const renderNodes = treeToRenderNodes(rootElement);

      // Send to native for rendering
      if (renderNodes.length > 0) {
        // Use renderTree which handles layout and painting
        native.renderTree(renderNodes as any);

        // Flush to display
        native.flushTerminal();
      }
    } catch (error) {
      if (onError) {
        onError(error as Error);
      } else {
        console.error("Render error:", error);
      }
    }
  }

  async function getTerminalInfo() {
    const n = await loadNative();
    const info = n.getTerminalInfo();
    return { width: info.width, height: info.height };
  }

  async function eventLoop() {
    const n = await loadNative();

    while (running) {
      try {
        const event = n.pollEvent(16); // ~60fps

        if (event) {
          // Handle resize
          if (event.eventType === "resize") {
            n.syncTerminalSize();
            n.clearScreen();
            needsRender = true;
          }

          // Handle Ctrl+C
          if (
            exitOnCtrlC &&
            event.eventType === "key" &&
            event.char === "c" &&
            event.modifiers?.ctrl
          ) {
            await unmount();
            break;
          }

          // Dispatch key events
          if (event.eventType === "key") {
            lastKeyEvent.value = {
              type: "key",
              key: event.key ?? undefined,
              char: event.char ?? undefined,
              ctrl: event.modifiers?.ctrl ?? false,
              alt: event.modifiers?.alt ?? false,
              shift: event.modifiers?.shift ?? false,
            };
          }
        }

        // Render frame if needed
        if (needsRender) {
          render();
          needsRender = false;
        }

        // Schedule re-render (Vue reactivity will trigger updates)
        needsRender = true;
      } catch (error) {
        if (onError) {
          onError(error as Error);
        }
      }

      // Small delay to prevent busy loop
      await new Promise((resolve) => setTimeout(resolve, 16));
    }
  }

  return {
    mount,
    unmount,
    waitUntilExit,
    render,
    getTerminalInfo,
  };
}
