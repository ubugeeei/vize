/**
 * Tree Component - Tree view for hierarchical data
 */

import { defineComponent, h, type PropType, type VNode } from "@vue/runtime-core";

export interface TreeNode {
  key: string;
  label: string;
  children?: TreeNode[];
  icon?: string;
  disabled?: boolean;
}

export interface TreeProps {
  /** Tree data */
  data: TreeNode[];
  /** Expanded node keys */
  expanded?: string[];
  /** Selected node key */
  selected?: string;
  /** Show lines */
  showLines?: boolean;
  /** Indent size */
  indent?: number;
  /** Expanded icon */
  expandedIcon?: string;
  /** Collapsed icon */
  collapsedIcon?: string;
  /** Leaf icon */
  leafIcon?: string;
  /** Foreground color */
  fg?: string;
  /** Selected foreground color */
  selectedFg?: string;
}

export const Tree = defineComponent({
  name: "Tree",
  props: {
    data: {
      type: Array as PropType<TreeNode[]>,
      required: true,
    },
    expanded: {
      type: Array as PropType<string[]>,
      default: () => [],
    },
    selected: String,
    showLines: {
      type: Boolean,
      default: true,
    },
    indent: {
      type: Number,
      default: 2,
    },
    expandedIcon: {
      type: String,
      default: "▼",
    },
    collapsedIcon: {
      type: String,
      default: "▶",
    },
    leafIcon: {
      type: String,
      default: "•",
    },
    fg: String,
    selectedFg: {
      type: String,
      default: "cyan",
    },
  },
  emits: ["select", "toggle"],
  setup(props, { emit: _emit }) {
    const renderNode = (
      node: TreeNode,
      depth: number,
      isLast: boolean,
      prefix: string,
    ): VNode[] => {
      const nodes: VNode[] = [];
      const hasChildren = node.children && node.children.length > 0;
      const isExpanded = props.expanded?.includes(node.key);
      const isSelected = node.key === props.selected;

      // Build line prefix
      let linePrefix = prefix;
      if (props.showLines && depth > 0) {
        linePrefix += isLast ? "└─" : "├─";
      }

      // Icon
      let icon = props.leafIcon;
      if (hasChildren) {
        icon = isExpanded ? props.expandedIcon : props.collapsedIcon;
      }
      if (node.icon) {
        icon = node.icon;
      }

      // Node line
      nodes.push(
        h(
          "text",
          {
            key: node.key,
            fg: isSelected ? props.selectedFg : props.fg,
            bold: isSelected,
            dim: node.disabled,
          },
          `${linePrefix}${icon} ${node.label}`,
        ),
      );

      // Children
      if (hasChildren && isExpanded) {
        const childPrefix = prefix + (props.showLines && depth > 0 ? (isLast ? "  " : "│ ") : "");
        node.children!.forEach((child, index) => {
          const childIsLast = index === node.children!.length - 1;
          nodes.push(
            ...renderNode(child, depth + 1, childIsLast, childPrefix + " ".repeat(props.indent)),
          );
        });
      }

      return nodes;
    };

    return () => {
      const children: VNode[] = [];

      props.data.forEach((node, index) => {
        const isLast = index === props.data.length - 1;
        children.push(...renderNode(node, 0, isLast, ""));
      });

      return h("box", { style: { flex_direction: "column" } }, children);
    };
  },
});
