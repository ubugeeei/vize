import type { ComputedRef } from 'vue'
import { createContext } from '../shared'

export type GraphDataPoint = Record<string, unknown>

export interface GraphRootProps {
  width?: number | string
  height?: number | string
  padding?: { top?: number; right?: number; bottom?: number; left?: number }
  data: GraphDataPoint[]
  xAccessor?: string | ((d: GraphDataPoint) => number)
  yAccessor?: string | ((d: GraphDataPoint) => number)
}

export interface ScaleLinear {
  (value: number): number
  domain: [number, number]
  range: [number, number]
  ticks: (count?: number) => number[]
  invert: (pixel: number) => number
}

export interface GraphRootContext {
  width: ComputedRef<number>
  height: ComputedRef<number>
  padding: { top: number; right: number; bottom: number; left: number }
  innerWidth: ComputedRef<number>
  innerHeight: ComputedRef<number>
  data: GraphDataPoint[]
  xScale: ComputedRef<ScaleLinear>
  yScale: ComputedRef<ScaleLinear>
  getX: (d: GraphDataPoint) => number
  getY: (d: GraphDataPoint) => number
}

const _graphRootContext = createContext<GraphRootContext>('GraphRoot')
export const injectGraphRootContext = _graphRootContext[0]
export const provideGraphRootContext = _graphRootContext[1]

export interface GraphLineProps {
  color?: string
  strokeWidth?: number
  curve?: 'linear' | 'monotone'
}

export interface GraphBarProps {
  color?: string
  radius?: number
  gap?: number
}

export interface GraphAreaProps {
  color?: string
  opacity?: number
  curve?: 'linear' | 'monotone'
}

export interface GraphAxisProps {
  position: 'top' | 'right' | 'bottom' | 'left'
  tickCount?: number
  formatTick?: (value: number) => string
}

export interface GraphGridProps {
  horizontal?: boolean
  vertical?: boolean
  tickCount?: number
}

export interface GraphTooltipProps {
  snapToData?: boolean
}

export interface GraphDotProps {
  radius?: number
  color?: string
  activeRadius?: number
}
