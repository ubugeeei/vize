/**
 * Modal Component - Overlay dialog
 */

import { defineComponent, h, type PropType, type VNode } from "@vue/runtime-core";

export interface ModalProps {
  /** Whether the modal is visible */
  visible?: boolean;
  /** Modal title */
  title?: string;
  /** Modal width */
  width?: number | string;
  /** Modal height */
  height?: number | string;
  /** Border style */
  border?: "single" | "double" | "rounded" | "heavy";
  /** Title foreground color */
  titleFg?: string;
  /** Border foreground color */
  borderFg?: string;
  /** Background color */
  bg?: string;
}

export const Modal = defineComponent({
  name: "Modal",
  props: {
    visible: {
      type: Boolean,
      default: true,
    },
    title: String,
    width: {
      type: [Number, String] as PropType<number | string>,
      default: "50%",
    },
    height: {
      type: [Number, String] as PropType<number | string>,
      default: "auto",
    },
    border: {
      type: String as PropType<ModalProps["border"]>,
      default: "rounded",
    },
    titleFg: {
      type: String,
      default: "white",
    },
    borderFg: String,
    bg: String,
  },
  emits: ["close"],
  setup(props, { slots, emit: _emit }) {
    return () => {
      if (!props.visible) {
        return null;
      }

      const children: VNode[] = [];

      // Title bar
      if (props.title) {
        children.push(
          h(
            "box",
            {
              key: "title",
              style: {
                flex_direction: "row",
                justify_content: "center",
                padding_bottom: 1,
              },
            },
            [
              h(
                "text",
                {
                  bold: true,
                  fg: props.titleFg,
                },
                props.title,
              ),
            ],
          ),
        );
      }

      // Content
      children.push(
        h(
          "box",
          {
            key: "content",
            style: { flex_grow: 1 },
          },
          slots.default?.(),
        ),
      );

      // Modal container with centering
      return h(
        "box",
        {
          style: {
            justify_content: "center",
            align_items: "center",
            width: "100%",
            height: "100%",
          },
        },
        [
          h(
            "box",
            {
              style: {
                flex_direction: "column",
                width: String(props.width),
                height: String(props.height),
                padding: 1,
              },
              border: props.border,
              fg: props.borderFg,
              bg: props.bg,
            },
            children,
          ),
        ],
      );
    };
  },
});
