<script setup lang="ts">
import { ref, computed } from 'vue'
import { mdiChevronDown } from '@mdi/js'
import type { CapturedEvent } from '../composables/useEventCapture'
import MdiIcon from './MdiIcon.vue'

const props = defineProps<{
  events: CapturedEvent[]
  eventTypes: string[]
  eventCounts: Record<string, number>
  filterType: string
  isPaused: boolean
  currentVariantId?: string
}>()

const emit = defineEmits<{
  (e: 'clear'): void
  (e: 'filter', type: string): void
  (e: 'toggle-pause'): void
  (e: 'toggle-panel'): void
}>()

const selectedEvent = ref<CapturedEvent | null>(null)
const isCollapsed = ref(false)
const detailTab = ref<'info' | 'raw'>('info')

const displayEvents = computed(() => {
  return [...props.events].reverse()
})

const formatTimestamp = (ts: number) => {
  const date = new Date(ts)
  return date.toLocaleTimeString('en-US', {
    hour12: false,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    fractionalSecondDigits: 3,
  })
}

const formatPayload = (payload: unknown) => {
  if (payload === undefined || payload === null) return ''
  try {
    return JSON.stringify(payload, null, 2)
  } catch {
    return String(payload)
  }
}

const getEventTypeColor = (type: string) => {
  const colors: Record<string, string> = {
    click: '#60a5fa',
    input: '#4ade80',
    change: '#fbbf24',
    focus: '#a78bfa',
    blur: '#f472b6',
    keydown: '#f87171',
    keyup: '#fb923c',
    submit: '#22d3d8',
  }
  return colors[type] || '#9ca3af'
}

const toggleCollapse = () => {
  isCollapsed.value = !isCollapsed.value
  emit('toggle-panel')
}

const selectEvent = (event: CapturedEvent) => {
  if (selectedEvent.value?.id === event.id) {
    selectedEvent.value = null
  } else {
    selectedEvent.value = event
    detailTab.value = 'info'
  }
}

const formatRawValue = (value: unknown): string => {
  if (value === null) return 'null'
  if (value === undefined) return 'undefined'
  if (typeof value === 'boolean') return value ? 'true' : 'false'
  if (typeof value === 'number') return String(value)
  if (typeof value === 'string') return `"${value}"`
  return String(value)
}

const getRawValueClass = (value: unknown): string => {
  if (value === null || value === undefined) return 'raw-value--null'
  if (typeof value === 'boolean') return value ? 'raw-value--true' : 'raw-value--false'
  if (typeof value === 'number') return 'raw-value--number'
  if (typeof value === 'string') return 'raw-value--string'
  return ''
}
</script>

<template>
  <div :class="['event-panel', { 'event-panel--collapsed': isCollapsed }]">
    <!-- Header -->
    <div class="event-header">
      <div class="header-left">
        <button type="button" class="collapse-btn" @click="toggleCollapse" :title="isCollapsed ? 'Expand' : 'Collapse'">
          <MdiIcon :class="['collapse-icon', { 'collapse-icon--collapsed': isCollapsed }]" :path="mdiChevronDown" :size="14" />
        </button>
        <span class="header-title">Events</span>
        <span v-if="currentVariantId" class="current-variant-badge">{{ currentVariantId }}</span>
        <span class="event-count">{{ events.length }}</span>
      </div>

      <div class="header-controls">
        <!-- Filter -->
        <select
          class="filter-select"
          :value="filterType"
          @change="emit('filter', ($event.target as HTMLSelectElement).value)"
        >
          <option value="">All Events</option>
          <option v-for="type in eventTypes" :key="type" :value="type">
            {{ type }} ({{ eventCounts[type] || 0 }})
          </option>
        </select>

        <!-- Pause/Resume -->
        <button
          type="button"
          :class="['control-btn', { 'control-btn--active': isPaused }]"
          @click="emit('toggle-pause')"
          :title="isPaused ? 'Resume' : 'Pause'"
        >
          {{ isPaused ? '▶' : '⏸' }}
        </button>

        <!-- Clear -->
        <button
          type="button"
          class="control-btn control-btn--danger"
          @click="emit('clear')"
          title="Clear Events"
        >
          🗑
        </button>
      </div>
    </div>

    <!-- Event List -->
    <div v-if="!isCollapsed" class="event-content">
      <div class="event-list">
        <div
          v-for="event in displayEvents"
          :key="event.id"
          :class="['event-item', { 'event-item--selected': selectedEvent?.id === event.id }]"
          @click="selectEvent(event)"
        >
          <span class="event-time">{{ formatTimestamp(event.timestamp) }}</span>
          <span
            class="event-type"
            :style="{ '--event-color': getEventTypeColor(event.type) }"
          >
            {{ event.type }}
          </span>
          <span class="event-target">{{ event.target || '(unknown)' }}</span>
        </div>

        <div v-if="!events.length" class="event-empty">
          <span>No events captured</span>
        </div>
      </div>

      <!-- Event Detail -->
      <div v-if="selectedEvent" class="event-detail">
        <div class="detail-header">
          <span class="detail-title">Event Details</span>
          <button type="button" class="detail-close" @click="selectedEvent = null">×</button>
        </div>
        <div class="detail-tabs">
          <button
            type="button"
            :class="['detail-tab', { 'detail-tab--active': detailTab === 'info' }]"
            @click="detailTab = 'info'"
          >
            Info
          </button>
          <button
            type="button"
            :class="['detail-tab', { 'detail-tab--active': detailTab === 'raw' }]"
            @click="detailTab = 'raw'"
          >
            Raw
          </button>
        </div>
        <div class="detail-content">
          <template v-if="detailTab === 'info'">
            <div class="detail-row">
              <span class="detail-label">Type</span>
              <span class="detail-value detail-value--type" :style="{ '--event-color': getEventTypeColor(selectedEvent.type) }">{{ selectedEvent.type }}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Target</span>
              <span class="detail-value detail-value--mono">{{ selectedEvent.target || '(unknown)' }}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Time</span>
              <span class="detail-value">{{ formatTimestamp(selectedEvent.timestamp) }}</span>
            </div>
            <div class="detail-row">
              <span class="detail-label">Variant</span>
              <span class="detail-value">{{ selectedEvent.variantId }}</span>
            </div>
            <div v-if="selectedEvent.payload" class="detail-row detail-row--payload">
              <span class="detail-label">Payload</span>
              <pre class="detail-payload">{{ formatPayload(selectedEvent.payload) }}</pre>
            </div>
          </template>
          <template v-else-if="detailTab === 'raw'">
            <div v-if="selectedEvent.rawEvent" class="raw-event-grid">
              <template v-for="(value, key) in selectedEvent.rawEvent" :key="key">
                <div v-if="value !== undefined" class="raw-event-item">
                  <span class="raw-event-key">{{ key }}</span>
                  <span class="raw-event-value" :class="getRawValueClass(value)">{{ formatRawValue(value) }}</span>
                </div>
              </template>
            </div>
            <div v-else class="raw-event-empty">
              No raw event data available
            </div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.event-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  background: var(--musea-bg-secondary);
}

.event-panel--collapsed {
  height: auto;
}

.event-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: var(--musea-bg-tertiary);
  border-bottom: 1px solid var(--musea-border);
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.collapse-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  padding: 0;
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--musea-text-muted);
}

.collapse-icon {
  font-size: 0.625rem;
  transition: transform 0.15s;
}

.collapse-icon--collapsed {
  transform: rotate(-90deg);
}

.header-title {
  font-size: 0.75rem;
  font-weight: 600;
  color: var(--musea-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.current-variant-badge {
  padding: 0.125rem 0.5rem;
  background: var(--musea-accent-subtle);
  border-radius: 4px;
  font-size: 0.625rem;
  font-weight: 500;
  color: var(--musea-accent);
}

.event-count {
  padding: 0.0625rem 0.375rem;
  background: var(--musea-bg-primary);
  border-radius: 8px;
  font-size: 0.625rem;
  font-family: var(--musea-font-mono);
  color: var(--musea-text-muted);
}

.header-controls {
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

.filter-select {
  padding: 0.25rem 0.5rem;
  background: var(--musea-bg-primary);
  border: 1px solid var(--musea-border);
  border-radius: 3px;
  font-size: 0.6875rem;
  color: var(--musea-text-secondary);
  cursor: pointer;
}

.control-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  background: var(--musea-bg-primary);
  border: 1px solid var(--musea-border);
  border-radius: 3px;
  font-size: 0.75rem;
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all 0.15s;
}

.control-btn:hover {
  background: var(--musea-bg-secondary);
  color: var(--musea-text);
}

.control-btn--active {
  background: rgba(74, 222, 128, 0.15);
  border-color: #4ade80;
  color: #4ade80;
}

.control-btn--danger:hover {
  background: rgba(248, 113, 113, 0.15);
  border-color: #f87171;
  color: #f87171;
}

.event-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.event-list {
  flex: 1;
  overflow-y: auto;
  font-family: var(--musea-font-mono);
}

.event-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.375rem 0.75rem;
  border-bottom: 1px solid var(--musea-border);
  cursor: pointer;
  transition: background-color 0.1s;
}

.event-item:hover {
  background: var(--musea-bg-tertiary);
}

.event-item--selected {
  background: var(--musea-accent-subtle);
}

.event-time {
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
  flex-shrink: 0;
}

.event-type {
  font-size: 0.6875rem;
  padding: 0.0625rem 0.375rem;
  background: color-mix(in srgb, var(--event-color, #9ca3af) 20%, transparent);
  color: var(--event-color, #9ca3af);
  border-radius: 2px;
  font-weight: 500;
  flex-shrink: 0;
}

.event-target {
  font-size: 0.6875rem;
  color: var(--musea-text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.event-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem;
  color: var(--musea-text-muted);
  font-size: 0.75rem;
}

.event-detail {
  width: 280px;
  border-left: 1px solid var(--musea-border);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
}

.detail-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  background: var(--musea-bg-tertiary);
  border-bottom: 1px solid var(--musea-border);
}

.detail-title {
  font-size: 0.6875rem;
  font-weight: 600;
  color: var(--musea-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.detail-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  background: transparent;
  border: none;
  font-size: 1rem;
  color: var(--musea-text-muted);
  cursor: pointer;
}

.detail-close:hover {
  color: var(--musea-text);
}

.detail-content {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem;
}

.detail-row {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  padding: 0.375rem;
}

.detail-row--payload {
  flex: 1;
}

.detail-label {
  font-size: 0.5625rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--musea-text-muted);
}

.detail-value {
  font-size: 0.75rem;
  font-family: var(--musea-font-mono);
  color: var(--musea-text);
}

.detail-payload {
  margin: 0;
  padding: 0.5rem;
  background: var(--musea-bg-tertiary);
  border-radius: 4px;
  font-size: 0.6875rem;
  color: var(--musea-text-secondary);
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-all;
}

.detail-tabs {
  display: flex;
  border-bottom: 1px solid var(--musea-border);
  background: var(--musea-bg-tertiary);
}

.detail-tab {
  flex: 1;
  padding: 0.375rem 0.5rem;
  background: transparent;
  border: none;
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all 0.15s;
}

.detail-tab:hover {
  color: var(--musea-text-secondary);
}

.detail-tab--active {
  color: var(--musea-accent);
  background: var(--musea-bg-secondary);
}

.detail-value--type {
  padding: 0.0625rem 0.375rem;
  background: color-mix(in srgb, var(--event-color, #9ca3af) 20%, transparent);
  color: var(--event-color, #9ca3af);
  border-radius: 2px;
  font-weight: 500;
}

.detail-value--mono {
  font-family: var(--musea-font-mono);
  font-size: 0.6875rem;
  word-break: break-all;
}

.raw-event-grid {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.raw-event-item {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  padding: 0.25rem 0.375rem;
  border-radius: 3px;
}

.raw-event-item:hover {
  background: var(--musea-bg-tertiary);
}

.raw-event-key {
  font-size: 0.625rem;
  font-family: var(--musea-font-mono);
  color: var(--musea-text-muted);
  flex-shrink: 0;
  min-width: 80px;
}

.raw-event-value {
  font-size: 0.6875rem;
  font-family: var(--musea-font-mono);
  color: var(--musea-text);
  word-break: break-all;
}

.raw-value--null {
  color: var(--musea-text-muted);
  font-style: italic;
}

.raw-value--true {
  color: #4ade80;
}

.raw-value--false {
  color: #f87171;
}

.raw-value--number {
  color: #60a5fa;
}

.raw-value--string {
  color: #fbbf24;
}

.raw-event-empty {
  padding: 1rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.75rem;
}
</style>
