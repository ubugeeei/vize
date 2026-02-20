<script setup lang="ts">
import { computed, ref } from 'vue'
import hljs from 'highlight.js/lib/core'
import json from 'highlight.js/lib/languages/json'
import { mdiChevronUp, mdiChevronDown } from '@mdi/js'
import { useActions, type ActionEvent } from '../composables/useActions'
import MdiIcon from './MdiIcon.vue'

hljs.registerLanguage('json', json)

const { events, clear } = useActions()
const expandedIndex = ref<number | null>(null)

const reversedEvents = computed(() => [...events.value].reverse())

function toggleExpand(index: number) {
  expandedIndex.value = expandedIndex.value === index ? null : index
}

function formatTime(timestamp: number): string {
  const d = new Date(timestamp)
  return d.toLocaleTimeString('en', { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit', fractionalSecondDigits: 3 })
}

function formatRawEvent(event: ActionEvent): string {
  if (!event.rawEvent) {
    return JSON.stringify({ target: event.target, value: event.value }, null, 2)
  }
  return JSON.stringify(event.rawEvent, null, 2)
}

function highlightJson(str: string): string {
  return hljs.highlight(str, { language: 'json' }).value
}
</script>

<template>
  <div class="actions-panel">
    <div class="actions-header">
      <span class="actions-count">{{ events.length }} event{{ events.length !== 1 ? 's' : '' }}</span>
      <button v-if="events.length > 0" type="button" class="actions-clear-btn" @click="clear()">
        Clear
      </button>
    </div>

    <div v-if="events.length === 0" class="actions-empty">
      <p>No events captured yet.</p>
      <p class="actions-hint">Interact with the component to see events here.</p>
    </div>

    <div v-else class="actions-list">
      <div
        v-for="(event, index) in reversedEvents"
        :key="index"
        class="action-item"
        :class="{ expanded: expandedIndex === index }"
        @click="toggleExpand(index)"
      >
        <div class="action-row">
          <span class="action-time">{{ formatTime(event.timestamp) }}</span>
          <span class="action-type" :class="event.source">{{ event.name }}</span>
          <span v-if="event.target" class="action-target">{{ event.target }}</span>
          <MdiIcon
            class="action-expand-icon"
            :path="expandedIndex === index ? mdiChevronUp : mdiChevronDown"
            :size="12"
          />
        </div>
        <div v-if="expandedIndex === index" class="action-detail">
          <pre class="action-raw hljs"><code v-html="highlightJson(formatRawEvent(event))"></code></pre>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.actions-panel {
  font-size: 0.75rem;
}

.actions-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--musea-border);
  background: var(--musea-bg-secondary);
}

.actions-count {
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
}

.actions-clear-btn {
  padding: 0.125rem 0.375rem;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  font-size: 0.625rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.actions-clear-btn:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.actions-empty {
  padding: 1rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.75rem;
}

.actions-hint {
  font-size: 0.6875rem;
  margin-top: 0.25rem;
  opacity: 0.7;
}

.actions-list {
  max-height: 200px;
  overflow-y: auto;
}

.action-item {
  border-bottom: 1px solid var(--musea-border-subtle);
  cursor: pointer;
  transition: background var(--musea-transition);
}

.action-item:hover {
  background: var(--musea-bg-secondary);
}

.action-item.expanded {
  background: var(--musea-bg-secondary);
}

.action-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem;
  font-size: 0.6875rem;
}

.action-time {
  color: var(--musea-text-muted);
  font-family: var(--musea-font-mono, monospace);
  font-size: 0.625rem;
  flex-shrink: 0;
}

.action-type {
  padding: 0.0625rem 0.25rem;
  border-radius: 2px;
  font-size: 0.5625rem;
  font-weight: 600;
  flex-shrink: 0;
  background: rgba(59, 130, 246, 0.15);
  color: #60a5fa;
}

.action-type.vue {
  background: rgba(52, 211, 153, 0.15);
  color: #34d399;
}

.action-target {
  color: var(--musea-text-secondary);
  font-family: var(--musea-font-mono, monospace);
  font-size: 0.625rem;
}

.action-expand-icon {
  width: 12px;
  height: 12px;
  margin-left: auto;
  color: var(--musea-text-muted);
  flex-shrink: 0;
}

.action-detail {
  padding: 0 0.75rem 0.5rem 0.75rem;
}

.action-raw {
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: 0.375rem 0.5rem;
  font-family: var(--musea-font-mono, monospace);
  font-size: 0.5625rem;
  color: var(--musea-text-secondary);
  overflow-x: auto;
  white-space: pre;
  margin: 0;
  max-height: 150px;
  overflow-y: auto;
}
</style>
