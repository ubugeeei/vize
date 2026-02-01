/**
 * Tabs Component - Tab navigation
 */

import { defineComponent, h, type PropType, type VNode } from "@vue/runtime-core";

export interface Tab {
  key: string;
  label: string;
  disabled?: boolean;
}

export interface TabsProps {
  /** Tab definitions */
  tabs: Tab[];
  /** Currently active tab key */
  modelValue?: string;
  /** Tab bar position */
  position?: "top" | "bottom";
  /** Separator between tabs */
  separator?: string;
  /** Active tab foreground color */
  activeFg?: string;
  /** Active tab background color */
  activeBg?: string;
  /** Inactive tab foreground color */
  inactiveFg?: string;
  /** Show underline for active tab */
  underline?: boolean;
}

export const Tabs = defineComponent({
  name: "Tabs",
  props: {
    tabs: {
      type: Array as PropType<Tab[]>,
      required: true,
    },
    modelValue: String,
    position: {
      type: String as PropType<"top" | "bottom">,
      default: "top",
    },
    separator: {
      type: String,
      default: " | ",
    },
    activeFg: {
      type: String,
      default: "cyan",
    },
    activeBg: String,
    inactiveFg: String,
    underline: {
      type: Boolean,
      default: true,
    },
  },
  emits: ["update:modelValue", "change"],
  setup(props, { slots, emit: _emit }) {
    // Note: selectTab is designed for keyboard event handlers
    // It is currently not wired up but kept for future use

    return () => {
      // Tab bar
      const tabItems: VNode[] = [];

      props.tabs.forEach((tab, index) => {
        if (index > 0) {
          tabItems.push(h("text", { key: `sep-${index}`, dim: true }, props.separator));
        }

        const isActive = tab.key === props.modelValue;

        tabItems.push(
          h(
            "text",
            {
              key: tab.key,
              fg: isActive ? props.activeFg : props.inactiveFg,
              bg: isActive ? props.activeBg : undefined,
              bold: isActive,
              underline: isActive && props.underline,
              dim: tab.disabled,
            },
            tab.label,
          ),
        );
      });

      const tabBar = h(
        "box",
        {
          key: "tab-bar",
          style: {
            flex_direction: "row",
            padding_bottom: props.position === "top" ? 1 : 0,
            padding_top: props.position === "bottom" ? 1 : 0,
          },
        },
        tabItems,
      );

      // Content area
      const content = h(
        "box",
        {
          key: "content",
          style: { flex_grow: 1 },
        },
        slots.default?.(),
      );

      // Arrange based on position
      const children = props.position === "top" ? [tabBar, content] : [content, tabBar];

      return h(
        "box",
        {
          style: { flex_direction: "column", flex_grow: 1 },
        },
        children,
      );
    };
  },
});
