/**
 * Select Component - Dropdown/menu selection
 */

import { defineComponent, h, ref, type PropType, watch } from "@vue/runtime-core";

export interface SelectOption {
  label: string;
  value: string;
  disabled?: boolean;
}

export interface SelectProps {
  /** Options to display */
  options: SelectOption[];
  /** Currently selected value */
  modelValue?: string;
  /** Placeholder text */
  placeholder?: string;
  /** Whether the select is focused */
  focused?: boolean;
  /** Indicator for selected item */
  indicator?: string;
  /** Indicator for unselected item */
  indicatorEmpty?: string;
  /** Foreground color */
  fg?: string;
  /** Background color */
  bg?: string;
  /** Selected item foreground color */
  selectedFg?: string;
  /** Selected item background color */
  selectedBg?: string;
}

export const Select = defineComponent({
  name: "Select",
  props: {
    options: {
      type: Array as PropType<SelectOption[]>,
      required: true,
    },
    modelValue: String,
    placeholder: {
      type: String,
      default: "Select an option",
    },
    focused: {
      type: Boolean,
      default: false,
    },
    indicator: {
      type: String,
      default: "> ",
    },
    indicatorEmpty: {
      type: String,
      default: "  ",
    },
    fg: String,
    bg: String,
    selectedFg: {
      type: String,
      default: "cyan",
    },
    selectedBg: String,
  },
  emits: ["update:modelValue", "select"],
  setup(props, { emit: _emit }) {
    const highlightedIndex = ref(0);

    // Find initial index based on modelValue
    watch(
      () => props.modelValue,
      (value) => {
        if (value) {
          const index = props.options.findIndex((opt) => opt.value === value);
          if (index !== -1) {
            highlightedIndex.value = index;
          }
        }
      },
      { immediate: true },
    );

    // Note: selectOption, moveUp, moveDown are designed for keyboard event handlers
    // They are currently not wired up but kept for future use

    return () => {
      const children = props.options.map((option, index) => {
        const isHighlighted = index === highlightedIndex.value;
        const isSelected = option.value === props.modelValue;
        const indicator = isHighlighted ? props.indicator : props.indicatorEmpty;

        return h(
          "box",
          {
            key: option.value,
            style: { flex_direction: "row" },
          },
          [
            h(
              "text",
              {
                fg: isHighlighted ? props.selectedFg : props.fg,
                bg: isHighlighted ? props.selectedBg : props.bg,
                dim: option.disabled,
              },
              `${indicator}${option.label}${isSelected ? " (selected)" : ""}`,
            ),
          ],
        );
      });

      return h(
        "box",
        {
          style: { flex_direction: "column" },
          fg: props.fg,
          bg: props.bg,
        },
        children,
      );
    };
  },
});
