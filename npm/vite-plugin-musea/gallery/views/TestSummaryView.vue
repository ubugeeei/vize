<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { mdiPlay, mdiLoading, mdiCheckCircle, mdiCloseCircle, mdiCircleOutline, mdiAlertCircle, mdiOpenInNew } from '@mdi/js'
import { useArts } from '../composables/useArts'
import { useA11y, type A11yResult, type A11yViolation } from '../composables/useA11y'
import { getPreviewUrl } from '../api'
import MdiIcon from '../components/MdiIcon.vue'

const router = useRouter()
const { arts, load } = useArts()
const { init: initA11y, runA11y, getResult, isRunning: a11yIsRunning, results: a11yResults } = useA11y()

interface TestStatus {
  artPath: string
  artTitle: string
  variantName: string
  status: 'pending' | 'running' | 'passed' | 'failed'
  result?: A11yResult
}

const testQueue = ref<TestStatus[]>([])
const isRunningAll = ref(false)
const currentTestIndex = ref(-1)
const iframeRefs = ref<Map<string, HTMLIFrameElement>>(new Map())
const iframeReadyMap = ref<Map<string, boolean>>(new Map())

// Flatten all variants into test queue
const buildTestQueue = () => {
  const queue: TestStatus[] = []
  for (const art of arts.value) {
    for (const variant of art.variants) {
      const key = `${art.path}:${variant.name}`
      const existingResult = getResult(key)
      queue.push({
        artPath: art.path,
        artTitle: art.metadata.title || art.path,
        variantName: variant.name,
        status: existingResult ? (existingResult.violations.length > 0 ? 'failed' : 'passed') : 'pending',
        result: existingResult,
      })
    }
  }
  testQueue.value = queue
}

const summary = computed(() => {
  const total = testQueue.value.length
  const passed = testQueue.value.filter(t => t.status === 'passed').length
  const failed = testQueue.value.filter(t => t.status === 'failed').length
  const pending = testQueue.value.filter(t => t.status === 'pending').length
  const running = testQueue.value.filter(t => t.status === 'running').length

  let violations = 0
  let criticalCount = 0
  let seriousCount = 0
  let moderateCount = 0
  let minorCount = 0

  for (const test of testQueue.value) {
    if (test.result) {
      violations += test.result.violations.length
      for (const v of test.result.violations) {
        switch (v.impact) {
          case 'critical': criticalCount++; break
          case 'serious': seriousCount++; break
          case 'moderate': moderateCount++; break
          case 'minor': minorCount++; break
        }
      }
    }
  }

  return { total, passed, failed, pending, running, violations, criticalCount, seriousCount, moderateCount, minorCount }
})

const setIframeRef = (key: string, el: HTMLIFrameElement | null) => {
  if (el) {
    iframeRefs.value.set(key, el)
  }
}

const onIframeLoad = (key: string) => {
  iframeReadyMap.value.set(key, true)
}

// Run all tests sequentially
const runAllTests = async () => {
  if (isRunningAll.value) return

  isRunningAll.value = true

  // Reset all to pending
  for (const test of testQueue.value) {
    test.status = 'pending'
    test.result = undefined
  }

  for (let i = 0; i < testQueue.value.length; i++) {
    currentTestIndex.value = i
    const test = testQueue.value[i]
    const key = `${test.artPath}:${test.variantName}`

    test.status = 'running'

    // Wait for iframe to be ready
    const iframe = iframeRefs.value.get(key)
    if (!iframe) {
      test.status = 'failed'
      test.result = { violations: [{ id: 'error', impact: 'critical', description: 'Iframe not found', helpUrl: '', nodes: [] }], passes: 0, incomplete: 0 }
      continue
    }

    // Wait for iframe to load
    let waitCount = 0
    while (!iframeReadyMap.value.get(key) && waitCount < 50) {
      await new Promise(r => setTimeout(r, 100))
      waitCount++
    }

    if (!iframeReadyMap.value.get(key)) {
      test.status = 'failed'
      test.result = { violations: [{ id: 'error', impact: 'critical', description: 'Iframe load timeout', helpUrl: '', nodes: [] }], passes: 0, incomplete: 0 }
      continue
    }

    // Run a11y test
    runA11y(iframe, key)

    // Wait for result
    let resultWait = 0
    while (a11yIsRunning.value && resultWait < 100) {
      await new Promise(r => setTimeout(r, 100))
      resultWait++
    }

    const result = getResult(key)
    test.result = result
    test.status = result ? (result.violations.length > 0 ? 'failed' : 'passed') : 'failed'
  }

  currentTestIndex.value = -1
  isRunningAll.value = false
}

const getImpactColor = (impact: string): string => {
  switch (impact) {
    case 'critical': return '#f87171'
    case 'serious': return '#fb923c'
    case 'moderate': return '#fbbf24'
    case 'minor': return '#60a5fa'
    default: return '#7b8494'
  }
}

const getStatusIconPath = (status: TestStatus['status']) => {
  switch (status) {
    case 'passed': return mdiCheckCircle
    case 'failed': return mdiCloseCircle
    case 'running': return mdiLoading
    default: return mdiCircleOutline
  }
}

const getStatusColor = (status: TestStatus['status']) => {
  switch (status) {
    case 'passed': return '#4ade80'
    case 'failed': return '#f87171'
    case 'running': return '#fbbf24'
    default: return '#7b8494'
  }
}

const navigateToComponent = (artPath: string) => {
  router.push({ name: 'component', params: { path: artPath } })
}

onMounted(() => {
  load()
  initA11y()
})

watch(arts, () => {
  buildTestQueue()
}, { immediate: true })

// Update test results when a11y results change
watch(a11yResults, () => {
  for (const test of testQueue.value) {
    const key = `${test.artPath}:${test.variantName}`
    const result = getResult(key)
    if (result && test.status !== 'running') {
      test.result = result
      test.status = result.violations.length > 0 ? 'failed' : 'passed'
    }
  }
}, { deep: true })
</script>

<template>
  <div class="test-summary">
    <div class="summary-header">
      <h1 class="summary-title">Test Summary</h1>
      <p class="summary-subtitle">Run accessibility tests on all components and variants</p>
    </div>

    <div class="summary-stats">
      <div class="stat total">
        <div class="stat-value">{{ summary.total }}</div>
        <div class="stat-label">Total Tests</div>
      </div>
      <div class="stat passed">
        <div class="stat-value">{{ summary.passed }}</div>
        <div class="stat-label">Passed</div>
      </div>
      <div class="stat failed">
        <div class="stat-value">{{ summary.failed }}</div>
        <div class="stat-label">Failed</div>
      </div>
      <div class="stat pending">
        <div class="stat-value">{{ summary.pending + summary.running }}</div>
        <div class="stat-label">Pending</div>
      </div>
      <div class="stat violations" v-if="summary.violations > 0">
        <div class="stat-value">{{ summary.violations }}</div>
        <div class="stat-label">Violations</div>
      </div>
    </div>

    <div v-if="summary.violations > 0" class="violation-breakdown">
      <span v-if="summary.criticalCount > 0" class="violation-badge critical">
        {{ summary.criticalCount }} Critical
      </span>
      <span v-if="summary.seriousCount > 0" class="violation-badge serious">
        {{ summary.seriousCount }} Serious
      </span>
      <span v-if="summary.moderateCount > 0" class="violation-badge moderate">
        {{ summary.moderateCount }} Moderate
      </span>
      <span v-if="summary.minorCount > 0" class="violation-badge minor">
        {{ summary.minorCount }} Minor
      </span>
    </div>

    <div class="summary-actions">
      <button
        type="button"
        class="run-all-btn"
        :disabled="isRunningAll"
        @click="runAllTests"
      >
        <MdiIcon v-if="isRunningAll" class="spin" :path="mdiLoading" :size="16" />
        <MdiIcon v-else :path="mdiPlay" :size="16" />
        {{ isRunningAll ? `Running ${currentTestIndex + 1}/${testQueue.length}...` : 'Run All A11y Tests' }}
      </button>
    </div>

    <div class="test-list">
      <div
        v-for="(test, index) in testQueue"
        :key="`${test.artPath}:${test.variantName}`"
        class="test-item"
        :class="{
          running: test.status === 'running',
          passed: test.status === 'passed',
          failed: test.status === 'failed'
        }"
        @click="navigateToComponent(test.artPath)"
      >
        <MdiIcon
          class="test-status"
          :path="getStatusIconPath(test.status)"
          :size="18"
          :style="{ color: getStatusColor(test.status) }"
          :class="{ spin: test.status === 'running' }"
        />
        <div class="test-info">
          <div class="test-name">{{ test.artTitle }} / {{ test.variantName }}</div>
          <div v-if="test.result && test.result.violations.length > 0" class="test-violations">
            <span
              v-for="v in test.result.violations.slice(0, 3)"
              :key="v.id"
              class="violation-tag"
              :style="{ borderColor: getImpactColor(v.impact), color: getImpactColor(v.impact) }"
            >
              {{ v.id }}
            </span>
            <span v-if="test.result.violations.length > 3" class="more-violations">
              +{{ test.result.violations.length - 3 }} more
            </span>
          </div>
        </div>
        <div v-if="test.result" class="test-counts">
          <span v-if="test.result.violations.length > 0" class="count violations">
            {{ test.result.violations.length }} issues
          </span>
          <span class="count passes">{{ test.result.passes }} passed</span>
        </div>
        <MdiIcon class="test-nav-icon" :path="mdiOpenInNew" :size="14" />
      </div>
    </div>

    <!-- Hidden iframes for testing -->
    <div class="hidden-iframes">
      <iframe
        v-for="test in testQueue"
        :key="`iframe-${test.artPath}:${test.variantName}`"
        :ref="(el) => setIframeRef(`${test.artPath}:${test.variantName}`, el as HTMLIFrameElement)"
        :src="getPreviewUrl(test.artPath, test.variantName)"
        @load="onIframeLoad(`${test.artPath}:${test.variantName}`)"
      />
    </div>
  </div>
</template>

<style scoped>
.test-summary {
  padding: 1.5rem;
  max-width: 1000px;
  margin: 0 auto;
}

.summary-header {
  margin-bottom: 1.5rem;
}

.summary-title {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 0.25rem;
}

.summary-subtitle {
  color: var(--musea-text-muted);
  font-size: 0.875rem;
}

.summary-stats {
  display: flex;
  gap: 1rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.stat {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  padding: 1rem 1.5rem;
  text-align: center;
  min-width: 100px;
}

.stat-value {
  font-size: 1.75rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.stat-label {
  font-size: 0.75rem;
  color: var(--musea-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.stat.total .stat-value { color: var(--musea-text); }
.stat.passed .stat-value { color: #4ade80; }
.stat.failed .stat-value { color: #f87171; }
.stat.pending .stat-value { color: #7b8494; }
.stat.violations .stat-value { color: #fb923c; }

.violation-breakdown {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}

.violation-badge {
  padding: 0.25rem 0.5rem;
  border-radius: var(--musea-radius-sm);
  font-size: 0.75rem;
  font-weight: 600;
}

.violation-badge.critical { background: rgba(248, 113, 113, 0.15); color: #f87171; }
.violation-badge.serious { background: rgba(251, 146, 60, 0.15); color: #fb923c; }
.violation-badge.moderate { background: rgba(251, 191, 36, 0.15); color: #fbbf24; }
.violation-badge.minor { background: rgba(96, 165, 250, 0.15); color: #60a5fa; }

.summary-actions {
  margin-bottom: 1.5rem;
}

.run-all-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1.5rem;
  background: var(--musea-accent);
  border: none;
  border-radius: var(--musea-radius-md);
  color: #fff;
  font-size: 0.875rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.run-all-btn:hover:not(:disabled) {
  background: var(--musea-accent-hover);
}

.run-all-btn:disabled {
  opacity: 0.7;
  cursor: not-allowed;
}

.test-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.test-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  transition: all var(--musea-transition);
  cursor: pointer;
}

.test-item:hover {
  background: var(--musea-bg-tertiary);
  border-color: var(--musea-text-muted);
}

.test-nav-icon {
  color: var(--musea-text-muted);
  opacity: 0;
  transition: opacity var(--musea-transition);
  flex-shrink: 0;
}

.test-item:hover .test-nav-icon {
  opacity: 1;
}

.test-item.running {
  border-color: #fbbf24;
  background: rgba(251, 191, 36, 0.05);
}

.test-item.passed {
  border-color: rgba(74, 222, 128, 0.3);
}

.test-item.failed {
  border-color: rgba(248, 113, 113, 0.3);
}

.test-status {
  font-size: 1rem;
  font-weight: 700;
  width: 20px;
  text-align: center;
}

.test-info {
  flex: 1;
  min-width: 0;
}

.test-name {
  font-size: 0.875rem;
  font-weight: 500;
}

.test-violations {
  display: flex;
  gap: 0.375rem;
  margin-top: 0.375rem;
  flex-wrap: wrap;
}

.violation-tag {
  padding: 0.125rem 0.375rem;
  border: 1px solid;
  border-radius: var(--musea-radius-sm);
  font-size: 0.6875rem;
  font-family: var(--musea-font-mono);
}

.more-violations {
  color: var(--musea-text-muted);
  font-size: 0.6875rem;
}

.test-counts {
  display: flex;
  gap: 0.75rem;
  font-size: 0.75rem;
}

.count {
  color: var(--musea-text-muted);
}

.count.violations {
  color: #f87171;
}

.count.passes {
  color: #4ade80;
}

.hidden-iframes {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  pointer-events: none;
  overflow: hidden;
}

.hidden-iframes iframe {
  width: 1280px;
  height: 720px;
  border: none;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.spin {
  animation: spin 1s linear infinite;
}
</style>
