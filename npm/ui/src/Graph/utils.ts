import type { ScaleLinear } from './types'

export function extent(data: Record<string, unknown>[], accessor: (d: Record<string, unknown>) => number): [number, number] {
  let min = Infinity
  let max = -Infinity
  for (const d of data) {
    const v = accessor(d)
    if (v < min) min = v
    if (v > max) max = v
  }
  if (min === Infinity) return [0, 0]
  return [min, max]
}

export function createLinearScale(domain: [number, number], range: [number, number]): ScaleLinear {
  const [d0, d1] = domain
  const [r0, r1] = range
  const domainSpan = d1 - d0 || 1

  const scale = ((value: number): number => {
    return r0 + ((value - d0) / domainSpan) * (r1 - r0)
  }) as ScaleLinear

  scale.domain = domain
  scale.range = range

  scale.invert = (pixel: number): number => {
    const rangeSpan = r1 - r0 || 1
    return d0 + ((pixel - r0) / rangeSpan) * (d1 - d0)
  }

  scale.ticks = (count: number = 5): number[] => {
    if (count <= 0) return []
    if (count === 1) return [(d0 + d1) / 2]

    const step = niceStep(domainSpan / (count - 1))
    const start = Math.ceil(d0 / step) * step
    const end = Math.floor(d1 / step) * step
    const ticks: number[] = []

    for (let v = start; v <= end + step * 0.5; v += step) {
      ticks.push(Math.round(v * 1e12) / 1e12)
    }
    return ticks
  }

  return scale
}

function niceStep(rawStep: number): number {
  const magnitude = Math.pow(10, Math.floor(Math.log10(rawStep)))
  const residual = rawStep / magnitude
  let nice: number
  if (residual <= 1.5) nice = 1
  else if (residual <= 3) nice = 2
  else if (residual <= 7) nice = 5
  else nice = 10
  return nice * magnitude
}

export function linePath(points: [number, number][]): string {
  if (points.length === 0) return ''
  let d = `M${points[0][0]},${points[0][1]}`
  for (let i = 1; i < points.length; i++) {
    d += `L${points[i][0]},${points[i][1]}`
  }
  return d
}

export function monotonePath(points: [number, number][]): string {
  if (points.length === 0) return ''
  if (points.length === 1) return `M${points[0][0]},${points[0][1]}`
  if (points.length === 2) return linePath(points)

  // Monotone cubic Hermite interpolation (Fritsch-Carlson)
  const n = points.length
  const tangents: number[] = Array.from({ length: n })

  // Compute slopes
  const deltas: number[] = []
  const slopes: number[] = []
  for (let i = 0; i < n - 1; i++) {
    const dx = points[i + 1][0] - points[i][0]
    const dy = points[i + 1][1] - points[i][1]
    deltas.push(dx)
    slopes.push(dx === 0 ? 0 : dy / dx)
  }

  // Compute tangents
  tangents[0] = slopes[0]
  tangents[n - 1] = slopes[n - 2]
  for (let i = 1; i < n - 1; i++) {
    if (slopes[i - 1] * slopes[i] <= 0) {
      tangents[i] = 0
    } else {
      tangents[i] = (slopes[i - 1] + slopes[i]) / 2
    }
  }

  // Fritsch-Carlson monotonicity constraint
  for (let i = 0; i < n - 1; i++) {
    if (Math.abs(slopes[i]) < 1e-12) {
      tangents[i] = 0
      tangents[i + 1] = 0
    } else {
      const alpha = tangents[i] / slopes[i]
      const beta = tangents[i + 1] / slopes[i]
      const s = alpha * alpha + beta * beta
      if (s > 9) {
        const t = 3 / Math.sqrt(s)
        tangents[i] = t * alpha * slopes[i]
        tangents[i + 1] = t * beta * slopes[i]
      }
    }
  }

  // Build path
  let d = `M${points[0][0]},${points[0][1]}`
  for (let i = 0; i < n - 1; i++) {
    const dx = deltas[i] / 3
    const cp1x = points[i][0] + dx
    const cp1y = points[i][1] + dx * tangents[i]
    const cp2x = points[i + 1][0] - dx
    const cp2y = points[i + 1][1] - dx * tangents[i + 1]
    d += `C${cp1x},${cp1y},${cp2x},${cp2y},${points[i + 1][0]},${points[i + 1][1]}`
  }
  return d
}

export function areaPath(points: [number, number][], baseline: number): string {
  if (points.length === 0) return ''
  let d = `M${points[0][0]},${baseline}`
  d += `L${points[0][0]},${points[0][1]}`
  for (let i = 1; i < points.length; i++) {
    d += `L${points[i][0]},${points[i][1]}`
  }
  d += `L${points[points.length - 1][0]},${baseline}`
  d += 'Z'
  return d
}

export function monotoneAreaPath(points: [number, number][], baseline: number): string {
  if (points.length === 0) return ''
  const upper = monotonePath(points)
  // Close the area by going along the baseline
  let d = upper
  d += `L${points[points.length - 1][0]},${baseline}`
  d += `L${points[0][0]},${baseline}`
  d += 'Z'
  return d
}
