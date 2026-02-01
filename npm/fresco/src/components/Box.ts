/**
 * Box Component - Container with flexbox layout
 */

import { defineComponent, h, type PropType } from "@vue/runtime-core";

export interface BoxProps {
  /** Flex direction */
  flexDirection?: "row" | "column" | "row-reverse" | "column-reverse";
  /** Flex wrap */
  flexWrap?: "nowrap" | "wrap" | "wrap-reverse";
  /** Justify content */
  justifyContent?:
    | "flex-start"
    | "flex-end"
    | "center"
    | "space-between"
    | "space-around"
    | "space-evenly";
  /** Align items */
  alignItems?: "flex-start" | "flex-end" | "center" | "stretch" | "baseline";
  /** Align self */
  alignSelf?: "auto" | "flex-start" | "flex-end" | "center" | "stretch" | "baseline";
  /** Flex grow */
  flexGrow?: number;
  /** Flex shrink */
  flexShrink?: number;
  /** Width */
  width?: number | string;
  /** Height */
  height?: number | string;
  /** Min width */
  minWidth?: number | string;
  /** Min height */
  minHeight?: number | string;
  /** Max width */
  maxWidth?: number | string;
  /** Max height */
  maxHeight?: number | string;
  /** Padding (all sides) */
  padding?: number;
  /** Padding X (left and right) */
  paddingX?: number;
  /** Padding Y (top and bottom) */
  paddingY?: number;
  /** Padding top */
  paddingTop?: number;
  /** Padding right */
  paddingRight?: number;
  /** Padding bottom */
  paddingBottom?: number;
  /** Padding left */
  paddingLeft?: number;
  /** Margin (all sides) */
  margin?: number;
  /** Margin X (left and right) */
  marginX?: number;
  /** Margin Y (top and bottom) */
  marginY?: number;
  /** Margin top */
  marginTop?: number;
  /** Margin right */
  marginRight?: number;
  /** Margin bottom */
  marginBottom?: number;
  /** Margin left */
  marginLeft?: number;
  /** Gap between children */
  gap?: number;
  /** Border style */
  border?: "none" | "single" | "double" | "rounded" | "heavy" | "dashed";
  /** Foreground color */
  fg?: string;
  /** Background color */
  bg?: string;
}

export const Box = defineComponent({
  name: "Box",
  props: {
    flexDirection: String as PropType<BoxProps["flexDirection"]>,
    flexWrap: String as PropType<BoxProps["flexWrap"]>,
    justifyContent: String as PropType<BoxProps["justifyContent"]>,
    alignItems: String as PropType<BoxProps["alignItems"]>,
    alignSelf: String as PropType<BoxProps["alignSelf"]>,
    flexGrow: Number,
    flexShrink: Number,
    width: [Number, String] as PropType<number | string>,
    height: [Number, String] as PropType<number | string>,
    minWidth: [Number, String] as PropType<number | string>,
    minHeight: [Number, String] as PropType<number | string>,
    maxWidth: [Number, String] as PropType<number | string>,
    maxHeight: [Number, String] as PropType<number | string>,
    padding: Number,
    paddingX: Number,
    paddingY: Number,
    paddingTop: Number,
    paddingRight: Number,
    paddingBottom: Number,
    paddingLeft: Number,
    margin: Number,
    marginX: Number,
    marginY: Number,
    marginTop: Number,
    marginRight: Number,
    marginBottom: Number,
    marginLeft: Number,
    gap: Number,
    border: String as PropType<BoxProps["border"]>,
    fg: String,
    bg: String,
  },
  setup(props, { slots }) {
    return () => {
      const style: Record<string, unknown> = {};

      // Layout props
      if (props.flexDirection) style.flex_direction = props.flexDirection;
      if (props.flexWrap) style.flex_wrap = props.flexWrap;
      if (props.justifyContent) style.justify_content = props.justifyContent;
      if (props.alignItems) style.align_items = props.alignItems;
      if (props.flexGrow !== undefined) style.flex_grow = props.flexGrow;
      if (props.flexShrink !== undefined) style.flex_shrink = props.flexShrink;

      // Dimensions
      if (props.width !== undefined) style.width = String(props.width);
      if (props.height !== undefined) style.height = String(props.height);
      if (props.minWidth !== undefined) style.min_width = String(props.minWidth);
      if (props.minHeight !== undefined) style.min_height = String(props.minHeight);
      if (props.maxWidth !== undefined) style.max_width = String(props.maxWidth);
      if (props.maxHeight !== undefined) style.max_height = String(props.maxHeight);

      // Padding
      if (props.padding !== undefined) style.padding = props.padding;
      if (props.paddingTop !== undefined || props.paddingY !== undefined) {
        style.padding_top = props.paddingTop ?? props.paddingY ?? props.padding;
      }
      if (props.paddingRight !== undefined || props.paddingX !== undefined) {
        style.padding_right = props.paddingRight ?? props.paddingX ?? props.padding;
      }
      if (props.paddingBottom !== undefined || props.paddingY !== undefined) {
        style.padding_bottom = props.paddingBottom ?? props.paddingY ?? props.padding;
      }
      if (props.paddingLeft !== undefined || props.paddingX !== undefined) {
        style.padding_left = props.paddingLeft ?? props.paddingX ?? props.padding;
      }

      // Margin
      if (props.margin !== undefined) style.margin = props.margin;
      if (props.marginTop !== undefined || props.marginY !== undefined) {
        style.margin_top = props.marginTop ?? props.marginY ?? props.margin;
      }
      if (props.marginRight !== undefined || props.marginX !== undefined) {
        style.margin_right = props.marginRight ?? props.marginX ?? props.margin;
      }
      if (props.marginBottom !== undefined || props.marginY !== undefined) {
        style.margin_bottom = props.marginBottom ?? props.marginY ?? props.margin;
      }
      if (props.marginLeft !== undefined || props.marginX !== undefined) {
        style.margin_left = props.marginLeft ?? props.marginX ?? props.margin;
      }

      // Gap
      if (props.gap !== undefined) style.gap = props.gap;

      return h(
        "box",
        {
          style,
          border: props.border,
          fg: props.fg,
          bg: props.bg,
        },
        slots.default?.(),
      );
    };
  },
});
