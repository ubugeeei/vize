/**
 * Header Component - Application header
 */

import { defineComponent, h, type VNode } from "@vue/runtime-core";

export interface HeaderProps {
  /** Header title */
  title: string;
  /** Subtitle */
  subtitle?: string;
  /** Left content */
  left?: string;
  /** Right content */
  right?: string;
  /** Background color */
  bg?: string;
  /** Title foreground color */
  titleFg?: string;
  /** Subtitle foreground color */
  subtitleFg?: string;
  /** Border bottom */
  borderBottom?: boolean;
}

export const Header = defineComponent({
  name: "Header",
  props: {
    title: {
      type: String,
      required: true,
    },
    subtitle: String,
    left: String,
    right: String,
    bg: String,
    titleFg: {
      type: String,
      default: "white",
    },
    subtitleFg: {
      type: String,
      default: "gray",
    },
    borderBottom: {
      type: Boolean,
      default: false,
    },
  },
  setup(props, { slots }) {
    return () => {
      const leftContent = slots.left?.() ?? (props.left ? [h("text", {}, props.left)] : []);
      const rightContent = slots.right?.() ?? (props.right ? [h("text", {}, props.right)] : []);

      const centerContent = [
        h(
          "text",
          {
            fg: props.titleFg,
            bold: true,
          },
          props.title,
        ),
      ];

      if (props.subtitle) {
        centerContent.push(
          h(
            "text",
            {
              fg: props.subtitleFg,
              dim: true,
            },
            ` - ${props.subtitle}`,
          ),
        );
      }

      const children: VNode[] = [
        h(
          "box",
          {
            key: "header-content",
            bg: props.bg,
            style: {
              flex_direction: "row",
              justify_content: "space-between",
              align_items: "center",
              width: "100%",
              padding: 1,
            },
          },
          [
            h("box", { key: "left", style: { flex_direction: "row" } }, leftContent),
            h("box", { key: "center", style: { flex_direction: "row" } }, centerContent),
            h("box", { key: "right", style: { flex_direction: "row" } }, rightContent),
          ],
        ),
      ];

      if (props.borderBottom) {
        children.push(h("text", { key: "border", dim: true }, "─".repeat(80)));
      }

      return h("box", { style: { flex_direction: "column" } }, children);
    };
  },
});
