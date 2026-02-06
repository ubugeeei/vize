<script setup lang="ts">
import { computed, ref } from 'vue'
import { useActions, type ActionEvent } from '../composables/useActions'

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

function formatPayload(event: ActionEvent): string {
  const obj: Record<string, unknown> = {}
  if (event.target) obj.target = event.target
  if (event.value !== undefined) obj.value = event.value
  if (event.args !== undefined) obj.args = event.args
  return JSON.stringify(obj, null, 2)
}
</script>

<template>
  <div class="actions-panel">
    <div class="actions-header">
      <span class="actions-count">{{ events.length }} event{{ events.length !== 1 ? 's' : '' }}</span>
      <button v-if="events.length > 0" class="actions-clear-btn" @click="clear()">
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
        @click="toggleExpand(index)"
      >
        <div class="action-row">
          <span class="action-time">{{ formatTime(event.timestamp) }}</span>
          <span class="action-source" :class="event.source">{{ event.source }}</span>
          <span class="action-name">{{ event.name }}</span>
          <span v-if="event.target" class="action-target">&lt;{{ event.target }}&gt;</span>
        </div>
        <div v-if="expandedIndex === index" class="action-detail">
          <pre class="action-payload">{{ formatPayload(event) }}</pre>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.actions-panel {
  min-height: 200px;
}

.actions-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid var(--musea-border);
  background: var(--musea-bg-secondary);
  border-radius: var(--musea-radius-md) var(--musea-radius-md) 0 0;
}

.actions-count {
  font-size: 0.75rem;
  color: var(--musea-text-muted);
}

.actions-clear-btn {
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  background: var(--musea-bg-tertiary);
  color: var(--musea-text-muted);
  font-size: 0.6875rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.actions-clear-btn:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.actions-empty {
  padding: 2rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.875rem;
}

.actions-hint {
  font-size: 0.75rem;
  margin-top: 0.5rem;
  opacity: 0.7;
}

.actions-list {
  max-height: 400px;
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

.action-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  font-size: 0.75rem;
}

.action-time {
  color: var(--musea-text-muted);
  font-family: var(--musea-font-mono, monospace);
  font-size: 0.6875rem;
  flex-shrink: 0;
}

.action-source {
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  flex-shrink: 0;
}

.action-source.dom {
  background: rgba(59, 130, 246, 0.15);
  color: #60a5fa;
}

.action-source.vue {
  background: rgba(52, 211, 153, 0.15);
  color: #34d399;
}

.action-name {
  font-weight: 600;
  color: var(--musea-text);
}

.action-target {
  color: var(--musea-text-muted);
  font-family: var(--musea-font-mono, monospace);
  font-size: 0.6875rem;
}

.action-detail {
  padding: 0 1rem 0.5rem 1rem;
}

.action-payload {
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  padding: 0.5rem;
  font-family: var(--musea-font-mono, monospace);
  font-size: 0.6875rem;
  color: var(--musea-text-secondary);
  overflow-x: auto;
  white-space: pre;
}
</style>
