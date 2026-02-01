/**
 * Form Component - Form container with labels
 */

import { defineComponent, h, type PropType, type VNode } from "@vue/runtime-core";

export interface FormField {
  key: string;
  label: string;
  required?: boolean;
  hint?: string;
}

export interface FormProps {
  /** Form fields metadata */
  fields?: FormField[];
  /** Label width */
  labelWidth?: number;
  /** Gap between fields */
  gap?: number;
  /** Label position */
  labelPosition?: "left" | "top";
  /** Label foreground color */
  labelFg?: string;
  /** Required indicator */
  requiredIndicator?: string;
  /** Hint foreground color */
  hintFg?: string;
}

export const Form = defineComponent({
  name: "Form",
  props: {
    fields: {
      type: Array as PropType<FormField[]>,
      default: () => [],
    },
    labelWidth: {
      type: Number,
      default: 15,
    },
    gap: {
      type: Number,
      default: 1,
    },
    labelPosition: {
      type: String as PropType<"left" | "top">,
      default: "left",
    },
    labelFg: String,
    requiredIndicator: {
      type: String,
      default: "*",
    },
    hintFg: {
      type: String,
      default: "gray",
    },
  },
  setup(props, { slots }) {
    return () => {
      const children: VNode[] = [];

      props.fields.forEach((field, _index) => {
        const labelContent = [
          h(
            "text",
            {
              fg: props.labelFg,
            },
            field.label.padEnd(props.labelWidth),
          ),
        ];

        if (field.required) {
          labelContent.push(h("text", { fg: "red" }, props.requiredIndicator));
        }

        const fieldSlot = slots[field.key]?.();

        if (props.labelPosition === "top") {
          children.push(
            h(
              "box",
              {
                key: field.key,
                style: { flex_direction: "column", margin_bottom: props.gap },
              },
              [
                h("box", { style: { flex_direction: "row" } }, labelContent),
                fieldSlot ? h("box", { style: { margin_top: 0.5 } }, fieldSlot) : null,
                field.hint ? h("text", { fg: props.hintFg, dim: true }, field.hint) : null,
              ].filter(Boolean),
            ),
          );
        } else {
          children.push(
            h(
              "box",
              {
                key: field.key,
                style: {
                  flex_direction: "row",
                  align_items: "center",
                  margin_bottom: props.gap,
                },
              },
              [
                h(
                  "box",
                  { style: { width: String(props.labelWidth), flex_direction: "row" } },
                  labelContent,
                ),
                h("box", { style: { flex_grow: 1 } }, fieldSlot),
              ],
            ),
          );

          if (field.hint) {
            children.push(
              h(
                "text",
                {
                  key: `hint-${field.key}`,
                  fg: props.hintFg,
                  dim: true,
                  style: { margin_left: props.labelWidth, margin_bottom: props.gap },
                },
                field.hint,
              ),
            );
          }
        }
      });

      return h("box", { style: { flex_direction: "column" } }, children);
    };
  },
});
