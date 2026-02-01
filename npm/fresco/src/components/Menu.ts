/**
 * Menu Component - Command menu/palette
 */

import { defineComponent, h, type PropType, type VNode } from "@vue/runtime-core";

export interface MenuItem {
  key: string;
  label: string;
  shortcut?: string;
  icon?: string;
  disabled?: boolean;
  separator?: boolean;
}

export interface MenuProps {
  /** Menu items */
  items: MenuItem[];
  /** Focused item index */
  focusedIndex?: number;
  /** Show border */
  border?: "single" | "double" | "rounded" | "none";
  /** Width */
  width?: number;
  /** Foreground color */
  fg?: string;
  /** Focused foreground color */
  focusedFg?: string;
  /** Focused background color */
  focusedBg?: string;
  /** Shortcut foreground color */
  shortcutFg?: string;
}

export const Menu = defineComponent({
  name: "Menu",
  props: {
    items: {
      type: Array as PropType<MenuItem[]>,
      required: true,
    },
    focusedIndex: {
      type: Number,
      default: 0,
    },
    border: {
      type: String as PropType<MenuProps["border"]>,
      default: "single",
    },
    width: Number,
    fg: String,
    focusedFg: {
      type: String,
      default: "black",
    },
    focusedBg: {
      type: String,
      default: "cyan",
    },
    shortcutFg: {
      type: String,
      default: "gray",
    },
  },
  emits: ["select"],
  setup(props, { emit: _emit }) {
    return () => {
      const children: VNode[] = [];

      props.items.forEach((item, index) => {
        if (item.separator) {
          children.push(
            h("text", { key: `sep-${index}`, dim: true }, "─".repeat(props.width ?? 20)),
          );
          return;
        }

        const isFocused = index === props.focusedIndex;
        const itemContent: VNode[] = [];

        // Icon
        if (item.icon) {
          itemContent.push(h("text", { key: "icon" }, `${item.icon} `));
        }

        // Label
        itemContent.push(
          h(
            "text",
            {
              key: "label",
              fg: isFocused ? props.focusedFg : props.fg,
              bg: isFocused ? props.focusedBg : undefined,
              dim: item.disabled,
              style: { flex_grow: 1 },
            },
            item.label,
          ),
        );

        // Shortcut
        if (item.shortcut) {
          itemContent.push(
            h(
              "text",
              {
                key: "shortcut",
                fg: props.shortcutFg,
                dim: true,
              },
              `  ${item.shortcut}`,
            ),
          );
        }

        children.push(
          h(
            "box",
            {
              key: item.key,
              style: {
                flex_direction: "row",
                padding_left: 1,
                padding_right: 1,
              },
              bg: isFocused ? props.focusedBg : undefined,
            },
            itemContent,
          ),
        );
      });

      return h(
        "box",
        {
          border: props.border === "none" ? undefined : props.border,
          style: {
            flex_direction: "column",
            width: props.width ? String(props.width) : undefined,
          },
        },
        children,
      );
    };
  },
});
