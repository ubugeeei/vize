<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { mdiPlay, mdiLoading, mdiCheckCircle, mdiOpenInNew, mdiChevronDown, mdiChevronUp } from '@mdi/js'
import { useA11y, type A11yViolation, type A11yResult } from '../composables/useA11y'
import { getPreviewUrl } from '../api'
import MdiIcon from './MdiIcon.vue'

const props = defineProps<{
  artPath: string
  defaultVariantName?: string
}>()

const { isRunning, init, runA11y, getResult } = useA11y()

const iframeRef = ref<HTMLIFrameElement | null>(null)
const iframeReady = ref(false)
const hasRun = ref(false)
const expandedViolation = ref<string | null>(null)

const key = computed(() => `${props.artPath}:${props.defaultVariantName || 'default'}`)
const result = computed<A11yResult | undefined>(() => getResult(key.value))

const previewUrl = computed(() => {
  if (!props.defaultVariantName) return ''
  return getPreviewUrl(props.artPath, props.defaultVariantName)
})

onMounted(() => {
  init()
})

function onIframeLoad() {
  iframeReady.value = true
}

function runTest() {
  if (!iframeRef.value || !iframeReady.value) return
  hasRun.value = true
  runA11y(iframeRef.value, key.value)
}

function toggleViolation(id: string) {
  expandedViolation.value = expandedViolation.value === id ? null : id
}

function getImpactColor(impact: string): string {
  switch (impact) {
    case 'critical': return '#f87171'
    case 'serious': return '#fb923c'
    case 'moderate': return '#fbbf24'
    case 'minor': return '#60a5fa'
    default: return '#7b8494'
  }
}

const summary = computed(() => {
  if (!result.value) return null
  const violations = result.value.violations
  return {
    total: violations.length,
    critical: violations.filter(v => v.impact === 'critical').length,
    serious: violations.filter(v => v.impact === 'serious').length,
    moderate: violations.filter(v => v.impact === 'moderate').length,
    minor: violations.filter(v => v.impact === 'minor').length,
    passes: result.value.passes,
  }
})
</script>

<template>
  <div class="a11y-panel">
    <!-- Hidden iframe for testing -->
    <iframe
      v-if="previewUrl"
      ref="iframeRef"
      :src="previewUrl"
      class="a11y-iframe"
      @load="onIframeLoad"
    />

    <div class="a11y-header">
      <h3 class="a11y-title">Accessibility Test</h3>
      <button
        class="a11y-run-btn"
        :disabled="isRunning || !iframeReady"
        @click="runTest"
      >
        <MdiIcon v-if="isRunning" class="spin" :path="mdiLoading" :size="14" />
        <MdiIcon v-else :path="mdiPlay" :size="14" />
        {{ isRunning ? 'Running...' : 'Run Test' }}
      </button>
    </div>

    <div v-if="!hasRun" class="a11y-empty">
      <p>Click "Run Test" to check accessibility with axe-core.</p>
      <p class="a11y-hint">Tests WCAG 2.0/2.1 AA criteria and best practices.</p>
    </div>

    <template v-else-if="result">
      <div v-if="result.error" class="a11y-error">
        {{ result.error }}
      </div>

      <template v-else>
        <div v-if="summary" class="a11y-summary">
          <div class="a11y-stat" :class="{ 'has-issues': summary.total > 0 }">
            <span class="a11y-stat-value">{{ summary.total }}</span>
            <span class="a11y-stat-label">Violations</span>
          </div>
          <div class="a11y-stat critical" v-if="summary.critical > 0">
            <span class="a11y-stat-value">{{ summary.critical }}</span>
            <span class="a11y-stat-label">Critical</span>
          </div>
          <div class="a11y-stat serious" v-if="summary.serious > 0">
            <span class="a11y-stat-value">{{ summary.serious }}</span>
            <span class="a11y-stat-label">Serious</span>
          </div>
          <div class="a11y-stat moderate" v-if="summary.moderate > 0">
            <span class="a11y-stat-value">{{ summary.moderate }}</span>
            <span class="a11y-stat-label">Moderate</span>
          </div>
          <div class="a11y-stat minor" v-if="summary.minor > 0">
            <span class="a11y-stat-value">{{ summary.minor }}</span>
            <span class="a11y-stat-label">Minor</span>
          </div>
          <div class="a11y-stat passes">
            <span class="a11y-stat-value">{{ summary.passes }}</span>
            <span class="a11y-stat-label">Passes</span>
          </div>
        </div>

        <div v-if="result.violations.length === 0" class="a11y-success">
          <MdiIcon :path="mdiCheckCircle" :size="24" />
          <span>No accessibility violations found</span>
        </div>

        <div v-else class="a11y-violations">
          <div
            v-for="violation in result.violations"
            :key="violation.id"
            class="a11y-violation"
            :class="{ expanded: expandedViolation === violation.id }"
          >
            <div class="a11y-violation-header" @click="toggleViolation(violation.id)">
              <span class="a11y-impact" :style="{ color: getImpactColor(violation.impact) }">
                {{ violation.impact }}
              </span>
              <span class="a11y-rule-id">{{ violation.id }}</span>
              <span class="a11y-node-count">{{ violation.nodes.length }} element(s)</span>
              <MdiIcon
                class="a11y-expand-icon"
                :path="expandedViolation === violation.id ? mdiChevronUp : mdiChevronDown"
                :size="14"
              />
            </div>
            <div v-if="expandedViolation === violation.id" class="a11y-violation-detail">
              <p class="a11y-description">{{ violation.description }}</p>
              <a :href="violation.helpUrl" target="_blank" class="a11y-help-link">
                Learn more
                <MdiIcon :path="mdiOpenInNew" :size="12" />
              </a>
              <div class="a11y-nodes">
                <div v-for="(node, i) in violation.nodes" :key="i" class="a11y-node">
                  <pre class="a11y-node-html">{{ node.html }}</pre>
                  <p v-if="node.failureSummary" class="a11y-node-summary">{{ node.failureSummary }}</p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </template>
    </template>
  </div>
</template>

<style scoped>
.a11y-panel {
  padding: 0.5rem;
}

.a11y-iframe {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  pointer-events: none;
}

.a11y-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1rem;
}

.a11y-title {
  font-size: 0.875rem;
  font-weight: 600;
}

.a11y-run-btn {
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

.a11y-run-btn:hover:not(:disabled) {
  background: var(--musea-accent-hover);
}

.a11y-run-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.a11y-empty {
  padding: 2rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.875rem;
}

.a11y-hint {
  font-size: 0.75rem;
  margin-top: 0.5rem;
  opacity: 0.7;
}

.a11y-error {
  padding: 1rem;
  background: rgba(248, 113, 113, 0.1);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: var(--musea-radius-sm);
  color: #f87171;
  font-size: 0.8125rem;
}

.a11y-summary {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.a11y-stat {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: 0.5rem 0.75rem;
  text-align: center;
  min-width: 60px;
}

.a11y-stat-value {
  display: block;
  font-size: 1.25rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.a11y-stat-label {
  font-size: 0.625rem;
  color: var(--musea-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.a11y-stat.has-issues .a11y-stat-value { color: #f87171; }
.a11y-stat.critical .a11y-stat-value { color: #f87171; }
.a11y-stat.serious .a11y-stat-value { color: #fb923c; }
.a11y-stat.moderate .a11y-stat-value { color: #fbbf24; }
.a11y-stat.minor .a11y-stat-value { color: #60a5fa; }
.a11y-stat.passes .a11y-stat-value { color: #4ade80; }

.a11y-success {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem;
  background: rgba(74, 222, 128, 0.1);
  border: 1px solid rgba(74, 222, 128, 0.2);
  border-radius: var(--musea-radius-sm);
  color: #4ade80;
  font-weight: 500;
}

.a11y-violations {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.a11y-violation {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  overflow: hidden;
}

.a11y-violation-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  cursor: pointer;
  font-size: 0.75rem;
}

.a11y-violation-header:hover {
  background: var(--musea-bg-tertiary);
}

.a11y-impact {
  font-weight: 700;
  text-transform: uppercase;
  font-size: 0.625rem;
}

.a11y-rule-id {
  font-family: var(--musea-font-mono);
  color: var(--musea-text-secondary);
}

.a11y-node-count {
  color: var(--musea-text-muted);
  margin-left: auto;
}

.a11y-expand-icon {
  color: var(--musea-text-muted);
}

.a11y-violation-detail {
  padding: 0.75rem;
  border-top: 1px solid var(--musea-border);
  background: var(--musea-bg-tertiary);
}

.a11y-description {
  font-size: 0.8125rem;
  color: var(--musea-text-secondary);
  margin-bottom: 0.5rem;
}

.a11y-help-link {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.75rem;
  color: var(--musea-accent);
  text-decoration: none;
}

.a11y-help-link:hover {
  text-decoration: underline;
}

.a11y-nodes {
  margin-top: 0.75rem;
}

.a11y-node {
  background: var(--musea-bg-primary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: 0.5rem;
  margin-top: 0.5rem;
}

.a11y-node-html {
  font-family: var(--musea-font-mono);
  font-size: 0.6875rem;
  color: var(--musea-text-secondary);
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}

.a11y-node-summary {
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
  margin-top: 0.375rem;
  white-space: pre-wrap;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.spin {
  animation: spin 1s linear infinite;
}
</style>
