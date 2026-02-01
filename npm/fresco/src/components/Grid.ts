/**
 * Grid Component - Grid layout helper
 */

import { defineComponent, h, type VNode } from "@vue/runtime-core";

export interface GridProps {
  /** Number of columns */
  columns?: number;
  /** Gap between cells */
  gap?: number;
  /** Row gap */
  rowGap?: number;
  /** Column gap */
  columnGap?: number;
}

export const Grid = defineComponent({
  name: "Grid",
  props: {
    columns: {
      type: Number,
      default: 2,
    },
    gap: {
      type: Number,
      default: 1,
    },
    rowGap: Number,
    columnGap: Number,
  },
  setup(props, { slots }) {
    return () => {
      const children = slots.default?.() ?? [];
      const flatChildren = Array.isArray(children) ? children.flat() : [children];

      const rows: VNode[][] = [];
      let currentRow: VNode[] = [];

      flatChildren.forEach((child, index) => {
        currentRow.push(child);
        if (currentRow.length === props.columns || index === flatChildren.length - 1) {
          rows.push([...currentRow]);
          currentRow = [];
        }
      });

      const rowGap = props.rowGap ?? props.gap;
      const columnGap = props.columnGap ?? props.gap;

      return h(
        "box",
        {
          style: {
            flex_direction: "column",
            gap: rowGap,
          },
        },
        rows.map((row, rowIndex) =>
          h(
            "box",
            {
              key: `row-${rowIndex}`,
              style: {
                flex_direction: "row",
                gap: columnGap,
              },
            },
            row.map((cell, cellIndex) =>
              h(
                "box",
                {
                  key: `cell-${rowIndex}-${cellIndex}`,
                  style: { flex_grow: 1 },
                },
                [cell],
              ),
            ),
          ),
        ),
      );
    };
  },
});
