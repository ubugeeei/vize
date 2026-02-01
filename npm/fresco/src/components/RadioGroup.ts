/**
 * RadioGroup Component - Radio button group selection
 */

import { defineComponent, h, type PropType } from "@vue/runtime-core";

export interface RadioOption {
  label: string;
  value: string;
  disabled?: boolean;
}

export interface RadioGroupProps {
  /** Radio options */
  options: RadioOption[];
  /** Currently selected value */
  modelValue?: string;
  /** Layout direction */
  direction?: "horizontal" | "vertical";
  /** Focused option index */
  focusedIndex?: number;
  /** Selected indicator */
  selected?: string;
  /** Unselected indicator */
  unselected?: string;
  /** Foreground color */
  fg?: string;
  /** Selected foreground color */
  selectedFg?: string;
}

export const RadioGroup = defineComponent({
  name: "RadioGroup",
  props: {
    options: {
      type: Array as PropType<RadioOption[]>,
      required: true,
    },
    modelValue: String,
    direction: {
      type: String as PropType<"horizontal" | "vertical">,
      default: "vertical",
    },
    focusedIndex: Number,
    selected: {
      type: String,
      default: "◉",
    },
    unselected: {
      type: String,
      default: "○",
    },
    fg: String,
    selectedFg: {
      type: String,
      default: "green",
    },
  },
  emits: ["update:modelValue", "change"],
  setup(props, { emit: _emit }) {
    return () => {
      const children = props.options.map((option, index) => {
        const isSelected = option.value === props.modelValue;
        const isFocused = index === props.focusedIndex;
        const indicator = isSelected ? props.selected : props.unselected;

        return h(
          "text",
          {
            key: option.value,
            fg: isSelected ? props.selectedFg : props.fg,
            bold: isFocused,
            dim: option.disabled,
          },
          `${indicator} ${option.label}`,
        );
      });

      return h(
        "box",
        {
          style: {
            flex_direction: props.direction === "horizontal" ? "row" : "column",
            gap: props.direction === "horizontal" ? 2 : 0,
          },
        },
        children,
      );
    };
  },
});
