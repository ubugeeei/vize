<script setup lang="ts">
import { ref, computed } from 'vue'
import { mdiPlay, mdiLoading, mdiImageOutline } from '@mdi/js'
import { runVrt } from '../api'
import MdiIcon from './MdiIcon.vue'

const props = defineProps<{
  artPath: string
  defaultVariantName?: string
}>()

interface VrtResult {
  artPath: string
  variantName: string
  viewport: string
  passed: boolean
  isNew?: boolean
  diffPercentage?: number
  error?: string
}

interface VrtSummary {
  total: number
  passed: number
  failed: number
  new: number
}

const isRunning = ref(false)
const hasRun = ref(false)
const results = ref<VrtResult[]>([])
const summary = ref<VrtSummary | null>(null)
const error = ref<string | null>(null)
const updateSnapshots = ref(false)

const groupedResults = computed(() => {
  const groups: Record<string, VrtResult[]> = {}
  for (const r of results.value) {
    const key = r.variantName
    if (!groups[key]) groups[key] = []
    groups[key].push(r)
  }
  return groups
})

async function runTest() {
  isRunning.value = true
  error.value = null

  try {
    const data = await runVrt(props.artPath, updateSnapshots.value)
    results.value = data.results
    summary.value = data.summary
    hasRun.value = true
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    isRunning.value = false
  }
}

function getStatusIcon(result: VrtResult): string {
  if (result.error) return 'error'
  if (result.isNew) return 'new'
  if (result.passed) return 'pass'
  return 'fail'
}

function getStatusColor(result: VrtResult): string {
  if (result.error) return '#f87171'
  if (result.isNew) return '#60a5fa'
  if (result.passed) return '#4ade80'
  return '#f87171'
}
</script>

<template>
  <div class="vrt-panel">
    <div class="vrt-header">
      <h3 class="vrt-title">Visual Regression Testing</h3>
      <div class="vrt-actions">
        <label class="vrt-update-label">
          <input v-model="updateSnapshots" type="checkbox" class="vrt-checkbox" />
          Update snapshots
        </label>
        <button
          class="vrt-run-btn"
          :disabled="isRunning"
          @click="runTest"
        >
          <MdiIcon v-if="isRunning" class="spin" :path="mdiLoading" :size="14" />
          <MdiIcon v-else :path="mdiImageOutline" :size="14" />
          {{ isRunning ? 'Running...' : 'Run VRT' }}
        </button>
      </div>
    </div>

    <div v-if="!hasRun" class="vrt-empty">
      <p>Click "Run VRT" to capture and compare screenshots.</p>
      <p class="vrt-hint">Requires Playwright to be installed.</p>
    </div>

    <div v-else-if="error" class="vrt-error">
      <p>{{ error }}</p>
      <p class="vrt-hint">Make sure Playwright is installed: <code>npm install playwright</code></p>
    </div>

    <template v-else>
      <div v-if="summary" class="vrt-summary">
        <div class="vrt-stat total">
          <span class="vrt-stat-value">{{ summary.total }}</span>
          <span class="vrt-stat-label">Total</span>
        </div>
        <div class="vrt-stat passed">
          <span class="vrt-stat-value">{{ summary.passed }}</span>
          <span class="vrt-stat-label">Passed</span>
        </div>
        <div class="vrt-stat failed" v-if="summary.failed > 0">
          <span class="vrt-stat-value">{{ summary.failed }}</span>
          <span class="vrt-stat-label">Failed</span>
        </div>
        <div class="vrt-stat new" v-if="summary.new > 0">
          <span class="vrt-stat-value">{{ summary.new }}</span>
          <span class="vrt-stat-label">New</span>
        </div>
      </div>

      <div class="vrt-results">
        <div v-for="(variantResults, variantName) in groupedResults" :key="variantName" class="vrt-variant">
          <div class="vrt-variant-name">{{ variantName }}</div>
          <div class="vrt-viewports">
            <div
              v-for="result in variantResults"
              :key="result.viewport"
              class="vrt-viewport"
              :class="getStatusIcon(result)"
            >
              <span class="vrt-viewport-name">{{ result.viewport }}</span>
              <span class="vrt-status" :style="{ color: getStatusColor(result) }">
                <template v-if="result.error">Error</template>
                <template v-else-if="result.isNew">New</template>
                <template v-else-if="result.passed">Pass</template>
                <template v-else>
                  Diff {{ result.diffPercentage?.toFixed(2) }}%
                </template>
              </span>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.vrt-panel {
  padding: 0.5rem;
}

.vrt-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1rem;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.vrt-title {
  font-size: 0.875rem;
  font-weight: 600;
}

.vrt-actions {
  display: flex;
  align-items: center;
  gap: 1rem;
}

.vrt-update-label {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  font-size: 0.75rem;
  color: var(--musea-text-muted);
  cursor: pointer;
}

.vrt-checkbox {
  width: 14px;
  height: 14px;
  cursor: pointer;
}

.vrt-run-btn {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.75rem;
  background: var(--musea-accent);
  border: none;
  border-radius: var(--musea-radius-sm);
  color: #fff;
  font-size: 0.75rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.vrt-run-btn:hover:not(:disabled) {
  background: var(--musea-accent-hover);
}

.vrt-run-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.vrt-empty {
  padding: 2rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.875rem;
}

.vrt-hint {
  font-size: 0.75rem;
  margin-top: 0.5rem;
  opacity: 0.7;
}

.vrt-hint code {
  background: var(--musea-bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
  font-family: var(--musea-font-mono);
}

.vrt-error {
  padding: 1rem;
  background: rgba(248, 113, 113, 0.1);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: var(--musea-radius-sm);
  color: #f87171;
  font-size: 0.8125rem;
}

.vrt-summary {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.vrt-stat {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: 0.5rem 0.75rem;
  text-align: center;
  min-width: 60px;
}

.vrt-stat-value {
  display: block;
  font-size: 1.25rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.vrt-stat-label {
  font-size: 0.625rem;
  color: var(--musea-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.vrt-stat.passed .vrt-stat-value { color: #4ade80; }
.vrt-stat.failed .vrt-stat-value { color: #f87171; }
.vrt-stat.new .vrt-stat-value { color: #60a5fa; }

.vrt-results {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.vrt-variant {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: 0.75rem;
}

.vrt-variant-name {
  font-weight: 600;
  font-size: 0.8125rem;
  margin-bottom: 0.5rem;
}

.vrt-viewports {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.vrt-viewport {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.25rem 0.5rem;
  background: var(--musea-bg-tertiary);
  border-radius: var(--musea-radius-sm);
  font-size: 0.75rem;
}

.vrt-viewport-name {
  color: var(--musea-text-secondary);
}

.vrt-status {
  font-weight: 600;
  font-size: 0.6875rem;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.spin {
  animation: spin 1s linear infinite;
}
</style>
