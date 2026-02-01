/**
 * Link Component - Clickable/styled link
 */

import { defineComponent, h } from "@vue/runtime-core";

export interface LinkProps {
  /** Link text */
  text: string;
  /** URL (for display, actual navigation not supported in TUI) */
  url?: string;
  /** Foreground color */
  fg?: string;
  /** Show underline */
  underline?: boolean;
  /** Show URL in parentheses */
  showUrl?: boolean;
}

export const Link = defineComponent({
  name: "Link",
  props: {
    text: {
      type: String,
      required: true,
    },
    url: String,
    fg: {
      type: String,
      default: "blue",
    },
    underline: {
      type: Boolean,
      default: true,
    },
    showUrl: {
      type: Boolean,
      default: false,
    },
  },
  emits: ["click"],
  setup(props, { emit: _emit }) {
    return () => {
      const parts = [
        h(
          "text",
          {
            fg: props.fg,
            underline: props.underline,
          },
          props.text,
        ),
      ];

      if (props.showUrl && props.url) {
        parts.push(h("text", { dim: true }, ` (${props.url})`));
      }

      return h("box", { style: { flex_direction: "row" } }, parts);
    };
  },
});
