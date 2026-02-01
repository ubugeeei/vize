/**
 * useInput - Input handling composable
 */

import { ref, watch, isRef, type Ref } from "@vue/runtime-core";
import { lastKeyEvent } from "../app.js";

export interface KeyHandler {
  (key: string, modifiers: { ctrl: boolean; alt: boolean; shift: boolean }): void;
}

export interface UseInputOptions {
  /** Whether to capture input (boolean or Ref<boolean>) */
  active?: boolean | Ref<boolean>;
  /** Whether to capture input (alias for active, boolean or Ref<boolean>) */
  isActive?: boolean | Ref<boolean>;
  /** Called on key press */
  onKey?: KeyHandler;
  /** Called on character input */
  onChar?: (char: string) => void;
  /** Called on Enter */
  onSubmit?: () => void;
  /** Called on Escape */
  onEscape?: () => void;
  /** Called on arrow keys */
  onArrow?: (direction: "up" | "down" | "left" | "right") => void;
}

export function useInput(options: UseInputOptions = {}) {
  const {
    active = true,
    isActive: isActiveOption,
    onKey,
    onChar,
    onSubmit,
    onEscape,
    onArrow,
  } = options;

  // Support both active and isActive, prefer isActive if both provided
  const activeSource = isActiveOption ?? active;
  const isActive = isRef(activeSource) ? activeSource : ref(activeSource);
  const lastKey = ref<string | null>(null);

  // Watch for key events from the app
  watch(lastKeyEvent, (event) => {
    if (!event || !isActive.value) return;

    const modifiers = {
      ctrl: event.ctrl,
      alt: event.alt,
      shift: event.shift,
    };

    // Character input
    if (event.char) {
      lastKey.value = event.char;
      onChar?.(event.char);
      onKey?.(event.char, modifiers);
      return;
    }

    // Special keys
    if (event.key) {
      lastKey.value = event.key;
      onKey?.(event.key, modifiers);

      switch (event.key) {
        case "enter":
          onSubmit?.();
          break;
        case "escape":
          onEscape?.();
          break;
        case "up":
        case "down":
        case "left":
        case "right":
          onArrow?.(event.key as "up" | "down" | "left" | "right");
          break;
      }
    }
  });

  const enable = () => {
    isActive.value = true;
  };

  const disable = () => {
    isActive.value = false;
  };

  return {
    isActive,
    lastKey,
    enable,
    disable,
  };
}

/**
 * Shorthand for handling specific key combinations
 */
export function useKeyPress(
  key: string,
  handler: () => void,
  options: { ctrl?: boolean; alt?: boolean; shift?: boolean } = {},
) {
  const { ctrl = false, alt = false, shift = false } = options;

  useInput({
    onKey: (pressedKey, modifiers) => {
      const matches =
        pressedKey.toLowerCase() === key.toLowerCase() &&
        modifiers.ctrl === ctrl &&
        modifiers.alt === alt &&
        modifiers.shift === shift;

      if (matches) {
        handler();
      }
    },
  });
}
