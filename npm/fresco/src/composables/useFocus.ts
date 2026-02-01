/**
 * useFocus - Focus management composable
 */

import { ref, computed, provide, inject, type InjectionKey, type Ref } from "@vue/runtime-core";

const FOCUS_KEY: InjectionKey<FocusManager> = Symbol("fresco-focus");

export interface UseFocusOptions {
  /** Whether this element starts focused */
  autoFocus?: boolean;
  /** Focus ID for this element */
  id?: string;
}

export interface FocusManager {
  /** Currently focused element ID */
  focusedId: Ref<string | null>;
  /** All focusable element IDs */
  focusableIds: Ref<string[]>;
  /** Focus a specific element */
  focus: (id: string) => void;
  /** Focus next element */
  focusNext: () => void;
  /** Focus previous element */
  focusPrevious: () => void;
  /** Register a focusable element */
  register: (id: string) => void;
  /** Unregister a focusable element */
  unregister: (id: string) => void;
}

/**
 * Create a focus manager (use at app root)
 */
export function createFocusManager(): FocusManager {
  const focusedId = ref<string | null>(null);
  const focusableIds = ref<string[]>([]);

  const focus = (id: string) => {
    if (focusableIds.value.includes(id)) {
      focusedId.value = id;
    }
  };

  const focusNext = () => {
    if (focusableIds.value.length === 0) return;

    const currentIndex = focusedId.value ? focusableIds.value.indexOf(focusedId.value) : -1;
    const nextIndex = (currentIndex + 1) % focusableIds.value.length;
    focusedId.value = focusableIds.value[nextIndex];
  };

  const focusPrevious = () => {
    if (focusableIds.value.length === 0) return;

    const currentIndex = focusedId.value
      ? focusableIds.value.indexOf(focusedId.value)
      : focusableIds.value.length;
    const prevIndex = (currentIndex - 1 + focusableIds.value.length) % focusableIds.value.length;
    focusedId.value = focusableIds.value[prevIndex];
  };

  const register = (id: string) => {
    if (!focusableIds.value.includes(id)) {
      focusableIds.value.push(id);
    }
  };

  const unregister = (id: string) => {
    const index = focusableIds.value.indexOf(id);
    if (index !== -1) {
      focusableIds.value.splice(index, 1);
      if (focusedId.value === id) {
        focusedId.value = focusableIds.value[0] ?? null;
      }
    }
  };

  return {
    focusedId,
    focusableIds,
    focus,
    focusNext,
    focusPrevious,
    register,
    unregister,
  };
}

/**
 * Provide focus manager to descendants
 */
export function provideFocusManager(manager: FocusManager) {
  provide(FOCUS_KEY, manager);
}

/**
 * Use focus management
 */
export function useFocus(options: UseFocusOptions = {}) {
  const { autoFocus = false, id = `focus-${Math.random().toString(36).slice(2)}` } = options;

  const manager = inject(FOCUS_KEY, null);
  const localFocused = ref(autoFocus);

  const isFocused = computed(() => {
    if (manager) {
      return manager.focusedId.value === id;
    }
    return localFocused.value;
  });

  const focus = () => {
    if (manager) {
      manager.focus(id);
    } else {
      localFocused.value = true;
    }
  };

  const blur = () => {
    if (manager) {
      if (manager.focusedId.value === id) {
        manager.focusedId.value = null;
      }
    } else {
      localFocused.value = false;
    }
  };

  // Register with manager
  if (manager) {
    manager.register(id);

    if (autoFocus && !manager.focusedId.value) {
      manager.focus(id);
    }
  }

  return {
    id,
    isFocused,
    focus,
    blur,
  };
}
