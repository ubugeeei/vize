/**
 * Breadcrumb Component - Navigation breadcrumb
 */

import { defineComponent, h, type PropType } from "@vue/runtime-core";

export interface BreadcrumbItem {
  key: string;
  label: string;
  icon?: string;
}

export interface BreadcrumbProps {
  /** Breadcrumb items */
  items: BreadcrumbItem[];
  /** Separator */
  separator?: string;
  /** Foreground color */
  fg?: string;
  /** Active (last item) foreground color */
  activeFg?: string;
  /** Separator foreground color */
  separatorFg?: string;
}

export const Breadcrumb = defineComponent({
  name: "Breadcrumb",
  props: {
    items: {
      type: Array as PropType<BreadcrumbItem[]>,
      required: true,
    },
    separator: {
      type: String,
      default: " > ",
    },
    fg: {
      type: String,
      default: "gray",
    },
    activeFg: {
      type: String,
      default: "white",
    },
    separatorFg: {
      type: String,
      default: "gray",
    },
  },
  emits: ["select"],
  setup(props, { emit: _emit }) {
    return () => {
      const children = props.items.flatMap((item, index) => {
        const isLast = index === props.items.length - 1;
        const result = [];

        // Icon
        if (item.icon) {
          result.push(
            h(
              "text",
              {
                key: `icon-${item.key}`,
                fg: isLast ? props.activeFg : props.fg,
              },
              `${item.icon} `,
            ),
          );
        }

        // Label
        result.push(
          h(
            "text",
            {
              key: item.key,
              fg: isLast ? props.activeFg : props.fg,
              bold: isLast,
              underline: !isLast,
            },
            item.label,
          ),
        );

        // Separator
        if (!isLast) {
          result.push(
            h(
              "text",
              {
                key: `sep-${item.key}`,
                fg: props.separatorFg,
              },
              props.separator,
            ),
          );
        }

        return result;
      });

      return h("box", { style: { flex_direction: "row" } }, children);
    };
  },
});
