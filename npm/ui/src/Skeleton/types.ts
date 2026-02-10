import type { PrimitiveProps } from '../Primitive'

export interface SkeletonProps extends PrimitiveProps {
  loading?: boolean
  animated?: boolean
  width?: string | number
  height?: string | number
  radius?: string | number
  circle?: boolean
}
