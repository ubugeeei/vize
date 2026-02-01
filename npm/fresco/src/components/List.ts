/**
 * List Component - Scrollable list of items
 */

import { defineComponent, h, ref, computed, type PropType, type VNode } from "@vue/runtime-core";

export interface ListItem {
  key: string;
  label: string;
  disabled?: boolean;
}

export interface ListProps {
  /** List items */
  items: ListItem[];
  /** Currently selected key */
  modelValue?: string;
  /** Maximum visible items (enables scrolling) */
  maxHeight?: number;
  /** Whether the list is focused */
  focused?: boolean;
  /** Item indicator */
  indicator?: string;
  /** Empty indicator */
  indicatorEmpty?: string;
  /** Foreground color */
  fg?: string;
  /** Selected foreground color */
  selectedFg?: string;
  /** Selected background color */
  selectedBg?: string;
}

export const List = defineComponent({
  name: "List",
  props: {
    items: {
      type: Array as PropType<ListItem[]>,
      required: true,
    },
    modelValue: String,
    maxHeight: Number,
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
    selectedFg: {
      type: String,
      default: "cyan",
    },
    selectedBg: String,
  },
  emits: ["update:modelValue", "select"],
  setup(props, { emit: _emit }) {
    const scrollOffset = ref(0);
    const highlightedIndex = ref(0);

    const visibleItems = computed(() => {
      if (!props.maxHeight) {
        return props.items;
      }
      const start = scrollOffset.value;
      const end = start + props.maxHeight;
      return props.items.slice(start, end);
    });

    const scrollIndicator = computed(() => {
      if (!props.maxHeight || props.items.length <= props.maxHeight) {
        return { showUp: false, showDown: false };
      }
      return {
        showUp: scrollOffset.value > 0,
        showDown: scrollOffset.value + props.maxHeight < props.items.length,
      };
    });

    return () => {
      const children: VNode[] = [];

      // Scroll up indicator
      if (scrollIndicator.value.showUp) {
        children.push(h("text", { key: "scroll-up", dim: true }, "  ..."));
      }

      // Visible items
      visibleItems.value.forEach((item, visibleIndex) => {
        const actualIndex = scrollOffset.value + visibleIndex;
        const isHighlighted = actualIndex === highlightedIndex.value;
        const isSelected = item.key === props.modelValue;
        const indicator = isHighlighted ? props.indicator : props.indicatorEmpty;

        children.push(
          h(
            "text",
            {
              key: item.key,
              fg: isHighlighted ? props.selectedFg : props.fg,
              bg: isHighlighted ? props.selectedBg : undefined,
              dim: item.disabled,
              bold: isSelected,
            },
            `${indicator}${item.label}`,
          ),
        );
      });

      // Scroll down indicator
      if (scrollIndicator.value.showDown) {
        children.push(h("text", { key: "scroll-down", dim: true }, "  ..."));
      }

      return h(
        "box",
        {
          style: { flex_direction: "column" },
          fg: props.fg,
        },
        children,
      );
    };
  },
});
