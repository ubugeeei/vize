/**
 * Text Component - Text display
 */

import { defineComponent, h } from "@vue/runtime-core";

export interface TextProps {
  /** Text content (alternative to slot) */
  content?: string;
  /** Enable text wrapping */
  wrap?: boolean;
  /** Foreground color */
  fg?: string;
  /** Background color */
  bg?: string;
  /** Bold text */
  bold?: boolean;
  /** Dim text */
  dim?: boolean;
  /** Italic text */
  italic?: boolean;
  /** Underlined text */
  underline?: boolean;
  /** Strikethrough text */
  strikethrough?: boolean;
}

export const Text = defineComponent({
  name: "Text",
  props: {
    content: String,
    wrap: Boolean,
    fg: String,
    bg: String,
    bold: Boolean,
    dim: Boolean,
    italic: Boolean,
    underline: Boolean,
    strikethrough: Boolean,
  },
  setup(props, { slots }) {
    return () => {
      // Get text from content prop or slot
      const text =
        props.content ??
        slots
          .default?.()
          ?.map((vnode) => {
            if (typeof vnode.children === "string") {
              return vnode.children;
            }
            return "";
          })
          .join("") ??
        "";

      return h("text", {
        text,
        wrap: props.wrap,
        fg: props.fg,
        bg: props.bg,
        bold: props.bold,
        dim: props.dim,
        italic: props.italic,
        underline: props.underline,
        strikethrough: props.strikethrough,
      });
    };
  },
});

/**
 * Convenience components for common text styles
 */

export const ErrorText = defineComponent({
  name: "ErrorText",
  props: {
    content: String,
  },
  setup(props, { slots }) {
    return () => h(Text, { fg: "red", ...props }, slots);
  },
});

export const WarningText = defineComponent({
  name: "WarningText",
  props: {
    content: String,
  },
  setup(props, { slots }) {
    return () => h(Text, { fg: "yellow", ...props }, slots);
  },
});

export const SuccessText = defineComponent({
  name: "SuccessText",
  props: {
    content: String,
  },
  setup(props, { slots }) {
    return () => h(Text, { fg: "green", ...props }, slots);
  },
});

export const InfoText = defineComponent({
  name: "InfoText",
  props: {
    content: String,
  },
  setup(props, { slots }) {
    return () => h(Text, { fg: "blue", ...props }, slots);
  },
});

export const MutedText = defineComponent({
  name: "MutedText",
  props: {
    content: String,
  },
  setup(props, { slots }) {
    return () => h(Text, { dim: true, ...props }, slots);
  },
});
