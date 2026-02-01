/**
 * Checkbox Component - Toggle checkbox
 */

import { defineComponent, h } from "@vue/runtime-core";

export interface CheckboxProps {
  /** Whether the checkbox is checked */
  modelValue?: boolean;
  /** Label text */
  label?: string;
  /** Whether the checkbox is focused */
  focused?: boolean;
  /** Whether the checkbox is disabled */
  disabled?: boolean;
  /** Checked indicator */
  checked?: string;
  /** Unchecked indicator */
  unchecked?: string;
  /** Foreground color */
  fg?: string;
  /** Checked foreground color */
  checkedFg?: string;
}

export const Checkbox = defineComponent({
  name: "Checkbox",
  props: {
    modelValue: {
      type: Boolean,
      default: false,
    },
    label: String,
    focused: {
      type: Boolean,
      default: false,
    },
    disabled: {
      type: Boolean,
      default: false,
    },
    checked: {
      type: String,
      default: "[x]",
    },
    unchecked: {
      type: String,
      default: "[ ]",
    },
    fg: String,
    checkedFg: {
      type: String,
      default: "green",
    },
  },
  emits: ["update:modelValue", "change"],
  setup(props, { emit: _emit }) {
    // Note: toggle is designed for keyboard event handlers
    // It is currently not wired up but kept for future use

    return () => {
      const indicator = props.modelValue ? props.checked : props.unchecked;
      const color = props.modelValue ? props.checkedFg : props.fg;

      return h(
        "box",
        {
          style: { flex_direction: "row" },
        },
        [
          h(
            "text",
            {
              fg: color,
              dim: props.disabled,
              bold: props.focused,
            },
            `${indicator} ${props.label ?? ""}`,
          ),
        ],
      );
    };
  },
});
