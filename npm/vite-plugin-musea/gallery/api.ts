import type { ArtFileInfo } from '../src/types.js'

export interface PaletteControl {
  name: string
  control: string
  default_value?: unknown
  description?: string
  required: boolean
  options: Array<{ label: string; value: unknown }>
  range?: { min: number; max: number; step?: number }
  group?: string
}

export interface PaletteApiResponse {
  title: string
  controls: PaletteControl[]
  groups: string[]
  json: string
  typescript: string
}

export interface AnalysisApiResponse {
  props: Array<{
    name: string
    type: string
    required: boolean
    default_value?: unknown
  }>
  emits: string[]
}

export interface DocApiResponse {
  markdown: string
  title: string
  category?: string
  variant_count: number
}

export interface A11yViolation {
  id: string
  impact: 'minor' | 'moderate' | 'serious' | 'critical'
  description: string
  helpUrl: string
  nodes: number
}

export interface A11yApiResponse {
  violations: A11yViolation[]
  passes: number
  incomplete: number
}

const basePath = (window as unknown as { __MUSEA_BASE_PATH__: string }).__MUSEA_BASE_PATH__ ?? '/__musea__'

async function fetchJson<T>(url: string): Promise<T> {
  const res = await fetch(basePath + url)
  if (!res.ok) {
    throw new Error(`API error: ${res.status} ${res.statusText}`)
  }
  return res.json() as Promise<T>
}

export async function fetchArts(): Promise<ArtFileInfo[]> {
  return fetchJson<ArtFileInfo[]>('/api/arts')
}

export async function fetchArt(artPath: string): Promise<ArtFileInfo> {
  return fetchJson<ArtFileInfo>(`/api/arts/${encodeURIComponent(artPath)}`)
}

export async function fetchPalette(artPath: string): Promise<PaletteApiResponse> {
  return fetchJson<PaletteApiResponse>(`/api/arts/${encodeURIComponent(artPath)}/palette`)
}

export async function fetchAnalysis(artPath: string): Promise<AnalysisApiResponse> {
  return fetchJson<AnalysisApiResponse>(`/api/arts/${encodeURIComponent(artPath)}/analysis`)
}

export async function fetchDocs(artPath: string): Promise<DocApiResponse> {
  return fetchJson<DocApiResponse>(`/api/arts/${encodeURIComponent(artPath)}/docs`)
}

export async function fetchA11y(artPath: string, variantName: string): Promise<A11yApiResponse> {
  return fetchJson<A11yApiResponse>(`/api/arts/${encodeURIComponent(artPath)}/variants/${encodeURIComponent(variantName)}/a11y`)
}

export function getPreviewUrl(artPath: string, variantName: string): string {
  return `${basePath}/preview?art=${encodeURIComponent(artPath)}&variant=${encodeURIComponent(variantName)}`
}

export function getBasePath(): string {
  return basePath
}

export interface VrtResult {
  artPath: string
  variantName: string
  viewport: string
  passed: boolean
  isNew?: boolean
  diffPercentage?: number
  error?: string
}

export interface VrtSummary {
  total: number
  passed: number
  failed: number
  new: number
}

export interface VrtApiResponse {
  success: boolean
  summary: VrtSummary
  results: VrtResult[]
}

export async function runVrt(artPath?: string, updateSnapshots?: boolean): Promise<VrtApiResponse> {
  const res = await fetch(basePath + '/api/run-vrt', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ artPath, updateSnapshots }),
  })
  if (!res.ok) {
    const data = await res.json()
    throw new Error(data.error || `API error: ${res.status}`)
  }
  return res.json() as Promise<VrtApiResponse>
}
