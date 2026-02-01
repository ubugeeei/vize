/**
 * Fresco Vue Custom Renderer
 */

import {
  createRenderer as createVueRenderer,
  type RendererOptions,
  type RendererNode,
  type RendererElement,
} from "@vue/runtime-core";

/**
 * Fresco node types
 */
export interface FrescoNode extends RendererNode {
  id: number;
  type: "box" | "text" | "input" | "root";
  props: Record<string, unknown>;
  children: FrescoNode[];
  parent: FrescoNode | null;
  text?: string;
}

/**
 * Fresco element (extends node)
 */
export interface FrescoElement extends FrescoNode, RendererElement {}

let nextId = 0;

function createNode(type: FrescoNode["type"]): FrescoNode {
  return {
    id: nextId++,
    type,
    props: {},
    children: [],
    parent: null,
  };
}

/**
 * Renderer options for Fresco
 */
const rendererOptions: RendererOptions<FrescoNode, FrescoElement> = {
  patchProp(el, key, prevValue, nextValue) {
    el.props[key] = nextValue;
  },

  insert(child, parent, anchor) {
    child.parent = parent;
    if (anchor) {
      const index = parent.children.indexOf(anchor);
      if (index !== -1) {
        parent.children.splice(index, 0, child);
        return;
      }
    }
    parent.children.push(child);
  },

  remove(child) {
    if (child.parent) {
      const index = child.parent.children.indexOf(child);
      if (index !== -1) {
        child.parent.children.splice(index, 1);
      }
      child.parent = null;
    }
  },

  createElement(type) {
    const nodeType = mapElementType(type);
    return createNode(nodeType) as FrescoElement;
  },

  createText(text) {
    const node = createNode("text");
    node.text = text;
    return node;
  },

  createComment() {
    // Comments are ignored in TUI
    return createNode("text");
  },

  setText(node, text) {
    node.text = text;
  },

  setElementText(el, text) {
    el.text = text;
    el.children = [];
  },

  parentNode(node) {
    return node.parent;
  },

  nextSibling(node) {
    if (!node.parent) return null;
    const index = node.parent.children.indexOf(node);
    return node.parent.children[index + 1] || null;
  },
};

/**
 * Map Vue element types to Fresco node types
 */
function mapElementType(type: string): FrescoNode["type"] {
  switch (type.toLowerCase()) {
    case "box":
    case "div":
    case "view":
      return "box";
    case "text":
    case "span":
      return "text";
    case "input":
    case "textinput":
      return "input";
    default:
      return "box";
  }
}

/**
 * Create the Fresco renderer
 */
export function createRenderer() {
  return createVueRenderer(rendererOptions);
}

/**
 * Convert Fresco tree to render nodes for native
 */
export function treeToRenderNodes(root: FrescoNode): Array<{
  id: number;
  nodeType: string;
  text?: string;
  wrap?: boolean;
  value?: string;
  placeholder?: string;
  focused?: boolean;
  mask?: boolean;
  style?: Record<string, unknown>;
  appearance?: Record<string, unknown>;
  border?: string;
  children?: number[];
}> {
  const nodes: Array<{
    id: number;
    nodeType: string;
    text?: string;
    wrap?: boolean;
    value?: string;
    placeholder?: string;
    focused?: boolean;
    mask?: boolean;
    style?: Record<string, unknown>;
    appearance?: Record<string, unknown>;
    border?: string;
    children?: number[];
  }> = [];

  function visit(node: FrescoNode) {
    const renderNode: (typeof nodes)[0] = {
      id: node.id,
      nodeType: node.type,
    };

    // Extract props
    if (node.text) {
      renderNode.text = node.text;
    }
    if (node.props.wrap !== undefined) {
      renderNode.wrap = Boolean(node.props.wrap);
    }
    if (node.props.value !== undefined) {
      const v = node.props.value;
      renderNode.value = typeof v === "string" || typeof v === "number" ? String(v) : "";
    }
    if (node.props.placeholder !== undefined) {
      const p = node.props.placeholder;
      renderNode.placeholder = typeof p === "string" || typeof p === "number" ? String(p) : "";
    }
    if (node.props.focused !== undefined) {
      renderNode.focused = Boolean(node.props.focused);
    }
    if (node.props.cursor !== undefined) {
      (renderNode as any).cursor = Number(node.props.cursor);
    }
    if (node.props.mask !== undefined) {
      renderNode.mask = Boolean(node.props.mask);
    }
    if (node.props.border !== undefined) {
      const b = node.props.border;
      renderNode.border = typeof b === "string" ? b : "";
    }

    // Extract style - only include defined values
    if (node.props.style) {
      const s = node.props.style as Record<string, unknown>;
      const style: Record<string, unknown> = {};

      if (s.display !== undefined) style.display = s.display;
      if (s.flexDirection !== undefined) style.flexDirection = s.flexDirection;
      if (s.flexWrap !== undefined) style.flexWrap = s.flexWrap;
      if (s.justifyContent !== undefined) style.justifyContent = s.justifyContent;
      if (s.alignItems !== undefined) style.alignItems = s.alignItems;
      if (s.alignSelf !== undefined) style.alignSelf = s.alignSelf;
      if (s.alignContent !== undefined) style.alignContent = s.alignContent;
      if (s.flexGrow !== undefined) style.flexGrow = s.flexGrow;
      if (s.flexShrink !== undefined) style.flexShrink = s.flexShrink;
      if (s.width !== undefined && (typeof s.width === "string" || typeof s.width === "number"))
        style.width = String(s.width);
      if (s.height !== undefined && (typeof s.height === "string" || typeof s.height === "number"))
        style.height = String(s.height);
      if (
        s.minWidth !== undefined &&
        (typeof s.minWidth === "string" || typeof s.minWidth === "number")
      )
        style.minWidth = String(s.minWidth);
      if (
        s.minHeight !== undefined &&
        (typeof s.minHeight === "string" || typeof s.minHeight === "number")
      )
        style.minHeight = String(s.minHeight);
      if (
        s.maxWidth !== undefined &&
        (typeof s.maxWidth === "string" || typeof s.maxWidth === "number")
      )
        style.maxWidth = String(s.maxWidth);
      if (
        s.maxHeight !== undefined &&
        (typeof s.maxHeight === "string" || typeof s.maxHeight === "number")
      )
        style.maxHeight = String(s.maxHeight);
      if (s.padding !== undefined) style.padding = s.padding;
      if (s.paddingTop !== undefined) style.paddingTop = s.paddingTop;
      if (s.paddingRight !== undefined) style.paddingRight = s.paddingRight;
      if (s.paddingBottom !== undefined) style.paddingBottom = s.paddingBottom;
      if (s.paddingLeft !== undefined) style.paddingLeft = s.paddingLeft;
      if (s.margin !== undefined) style.margin = s.margin;
      if (s.marginTop !== undefined) style.marginTop = s.marginTop;
      if (s.marginRight !== undefined) style.marginRight = s.marginRight;
      if (s.marginBottom !== undefined) style.marginBottom = s.marginBottom;
      if (s.marginLeft !== undefined) style.marginLeft = s.marginLeft;
      if (s.gap !== undefined) style.gap = s.gap;

      renderNode.style = style as any;
    }

    // Extract appearance (fg, bg, bold, etc.)
    const appearance: Record<string, unknown> = {};
    if (node.props.fg) appearance.fg = node.props.fg;
    if (node.props.bg) appearance.bg = node.props.bg;
    if (node.props.bold) appearance.bold = node.props.bold;
    if (node.props.dim) appearance.dim = node.props.dim;
    if (node.props.italic) appearance.italic = node.props.italic;
    if (node.props.underline) appearance.underline = node.props.underline;
    if (Object.keys(appearance).length > 0) {
      renderNode.appearance = appearance;
    }

    // Children
    if (node.children.length > 0) {
      renderNode.children = node.children.map((c) => c.id);
    }

    nodes.push(renderNode);

    // Visit children
    for (const child of node.children) {
      visit(child);
    }
  }

  visit(root);
  return nodes;
}
