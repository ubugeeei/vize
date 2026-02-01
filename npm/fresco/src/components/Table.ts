/**
 * Table Component - Display tabular data
 */

import { defineComponent, h, type PropType } from "@vue/runtime-core";

export interface TableColumn {
  /** Column key (maps to data property) */
  key: string;
  /** Column header text */
  header: string;
  /** Column width */
  width?: number | string;
  /** Text alignment */
  align?: "left" | "center" | "right";
}

export interface TableProps {
  /** Column definitions */
  columns: TableColumn[];
  /** Table data */
  data: Record<string, unknown>[];
  /** Show header row */
  showHeader?: boolean;
  /** Border style */
  border?: "none" | "single" | "double" | "rounded";
  /** Header foreground color */
  headerFg?: string;
  /** Header background color */
  headerBg?: string;
  /** Row foreground color */
  rowFg?: string;
  /** Alternate row background color */
  stripedBg?: string;
  /** Cell padding */
  cellPadding?: number;
}

export const Table = defineComponent({
  name: "Table",
  props: {
    columns: {
      type: Array as PropType<TableColumn[]>,
      required: true,
    },
    data: {
      type: Array as PropType<Record<string, unknown>[]>,
      required: true,
    },
    showHeader: {
      type: Boolean,
      default: true,
    },
    border: {
      type: String as PropType<TableProps["border"]>,
      default: "single",
    },
    headerFg: {
      type: String,
      default: "white",
    },
    headerBg: String,
    rowFg: String,
    stripedBg: String,
    cellPadding: {
      type: Number,
      default: 1,
    },
  },
  setup(props) {
    const formatCell = (value: unknown, width?: number | string): string => {
      const str =
        typeof value === "string" || typeof value === "number" || typeof value === "boolean"
          ? String(value)
          : value == null
            ? ""
            : JSON.stringify(value);
      if (typeof width === "number" && str.length < width) {
        return str.padEnd(width);
      }
      return str;
    };

    return () => {
      const rows: ReturnType<typeof h>[] = [];

      // Header row
      if (props.showHeader) {
        const headerCells = props.columns.map((col) =>
          h(
            "text",
            {
              key: col.key,
              bold: true,
              fg: props.headerFg,
              bg: props.headerBg,
            },
            formatCell(col.header, col.width),
          ),
        );

        rows.push(
          h(
            "box",
            {
              key: "header",
              style: {
                flex_direction: "row",
                gap: props.cellPadding,
              },
            },
            headerCells,
          ),
        );

        // Separator
        if (props.border !== "none") {
          const sepChar = props.border === "double" ? "=" : "-";
          const totalWidth = props.columns.reduce((acc, col) => {
            const w = typeof col.width === "number" ? col.width : 10;
            return acc + w + (props.cellPadding ?? 1);
          }, 0);

          rows.push(
            h(
              "text",
              {
                key: "separator",
                dim: true,
              },
              sepChar.repeat(totalWidth),
            ),
          );
        }
      }

      // Data rows
      props.data.forEach((row, rowIndex) => {
        const cells = props.columns.map((col) =>
          h(
            "text",
            {
              key: col.key,
              fg: props.rowFg,
              bg: rowIndex % 2 === 1 ? props.stripedBg : undefined,
            },
            formatCell(row[col.key], col.width),
          ),
        );

        rows.push(
          h(
            "box",
            {
              key: `row-${rowIndex}`,
              style: {
                flex_direction: "row",
                gap: props.cellPadding,
              },
            },
            cells,
          ),
        );
      });

      return h(
        "box",
        {
          style: { flex_direction: "column" },
          border: props.border,
        },
        rows,
      );
    };
  },
});
