<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import type { DesignToken, TokenUsageEntry } from '../../api'

defineProps<{
  isOpen: boolean
  tokenPath: string
  token?: DesignToken
  usages: TokenUsageEntry[]
}>()

const emit = defineEmits<{
  close: []
  editSource: [artPath: string]
}>()

const router = useRouter()
const expandedArts = ref<Set<string>>(new Set())

function toggleExpand(artPath: string) {
  if (expandedArts.value.has(artPath)) {
    expandedArts.value.delete(artPath)
  } else {
    expandedArts.value.add(artPath)
  }
}

function viewComponent(artPath: string) {
  emit('close')
  router.push({ name: 'component', params: { path: artPath } })
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="isOpen" class="modal-overlay" @click.self="emit('close')">
        <div class="modal-panel">
          <div class="modal-header">
            <div>
              <h2 class="modal-title">Token Usage</h2>
              <p class="modal-subtitle">
                <code>{{ tokenPath }}</code>
                <span v-if="token" class="modal-value">&mdash; {{ token.$resolvedValue ?? token.value }}</span>
              </p>
            </div>
            <button type="button" class="modal-close" @click="emit('close')">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          <div class="modal-body">
            <div v-if="token?.$tier === 'primitive' && usages.length > 0" class="primitive-warning">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
                <line x1="12" y1="9" x2="12" y2="13" />
                <line x1="12" y1="17" x2="12.01" y2="17" />
              </svg>
              <span>Components are referencing this primitive value directly. Consider using a semantic token instead.</span>
            </div>
            <div v-if="usages.length === 0" class="no-usage">
              No component usage found for this token value.
            </div>
            <div v-else class="usage-list">
              <div v-for="entry in usages" :key="entry.artPath" class="usage-entry">
                <div class="usage-entry-header" @click="toggleExpand(entry.artPath)">
                  <div class="usage-entry-info">
                    <span class="usage-entry-title">{{ entry.artTitle }}</span>
                    <span v-if="entry.artCategory" class="usage-category-badge">{{ entry.artCategory }}</span>
                    <span class="usage-match-count">{{ entry.matches.length }} match{{ entry.matches.length !== 1 ? 'es' : '' }}</span>
                  </div>
                  <svg
                    class="expand-icon"
                    :class="{ 'expand-icon--open': expandedArts.has(entry.artPath) }"
                    width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"
                  >
                    <polyline points="6 9 12 15 18 9" />
                  </svg>
                </div>

                <div v-if="expandedArts.has(entry.artPath)" class="usage-matches">
                  <div v-for="(match, idx) in entry.matches" :key="idx" class="usage-match-line">
                    <span class="match-line-number">{{ match.line }}</span>
                    <code class="match-line-content">{{ match.lineContent }}</code>
                    <span class="match-property">{{ match.property }}</span>
                  </div>
                </div>

                <div class="usage-entry-actions">
                  <button type="button" class="usage-action-btn" @click="viewComponent(entry.artPath)">
                    View Component
                  </button>
                  <button type="button" class="usage-action-btn usage-action-btn--edit" @click="emit('editSource', entry.artPath)">
                    Edit Source
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--musea-overlay);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 2rem;
}

.modal-panel {
  background: var(--musea-bg);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-lg, 12px);
  width: 100%;
  max-width: 640px;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: var(--musea-shadow);
}

.modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--musea-border);
}

.modal-title {
  font-size: 1rem;
  font-weight: 700;
  margin-bottom: 0.25rem;
}

.modal-subtitle {
  font-size: 0.75rem;
  color: var(--musea-text-muted);
}

.modal-subtitle code {
  font-family: var(--musea-font-mono);
  background: var(--musea-bg-secondary);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

.modal-value {
  color: var(--musea-text-muted);
}

.modal-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--musea-text-muted);
  border-radius: var(--musea-radius-sm, 4px);
  cursor: pointer;
  flex-shrink: 0;
}

.modal-close:hover {
  background: var(--musea-border);
  color: var(--musea-text);
}

.modal-body {
  overflow-y: auto;
  padding: 1rem 1.5rem;
}

.primitive-warning {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  margin-bottom: 0.75rem;
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.3);
  border-radius: var(--musea-radius-md);
  color: #f59e0b;
  font-size: 0.8125rem;
  line-height: 1.4;
}

.primitive-warning svg {
  flex-shrink: 0;
  margin-top: 0.125rem;
}

.no-usage {
  text-align: center;
  color: var(--musea-text-muted);
  padding: 2rem 0;
}

.usage-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.usage-entry {
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  overflow: hidden;
}

.usage-entry-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  cursor: pointer;
  transition: background var(--musea-transition);
}

.usage-entry-header:hover {
  background: var(--musea-bg-secondary);
}

.usage-entry-info {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.usage-entry-title {
  font-weight: 600;
  font-size: 0.875rem;
}

.usage-category-badge {
  font-size: 0.625rem;
  padding: 0.0625rem 0.375rem;
  border-radius: 9999px;
  background: rgba(59, 130, 246, 0.15);
  color: #60a5fa;
  text-transform: uppercase;
  font-weight: 600;
}

.usage-match-count {
  font-size: 0.75rem;
  color: var(--musea-text-muted);
}

.expand-icon {
  transition: transform 0.2s;
  flex-shrink: 0;
  color: var(--musea-text-muted);
}

.expand-icon--open {
  transform: rotate(180deg);
}

.usage-matches {
  border-top: 1px solid var(--musea-border);
  padding: 0.5rem 1rem;
  background: var(--musea-bg-secondary);
}

.usage-match-line {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
  padding: 0.25rem 0;
  font-size: 0.75rem;
}

.match-line-number {
  color: var(--musea-text-muted);
  font-family: var(--musea-font-mono);
  min-width: 2rem;
  text-align: right;
  flex-shrink: 0;
}

.match-line-content {
  font-family: var(--musea-font-mono);
  font-size: 0.75rem;
  flex: 1;
  min-width: 0;
  word-break: break-all;
}

.match-property {
  font-size: 0.625rem;
  color: var(--musea-accent);
  font-family: var(--musea-font-mono);
  flex-shrink: 0;
}

.usage-entry-actions {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border-top: 1px solid var(--musea-border);
}

.usage-action-btn {
  padding: 0.25rem 0.75rem;
  font-size: 0.75rem;
  font-weight: 500;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm, 4px);
  background: transparent;
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: border-color var(--musea-transition), color var(--musea-transition);
}

.usage-action-btn:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.usage-action-btn--edit:hover {
  border-color: var(--musea-accent);
  color: var(--musea-accent);
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-active .modal-panel,
.modal-leave-active .modal-panel {
  transition: transform 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-panel {
  transform: scale(0.95);
}

.modal-leave-to .modal-panel {
  transform: scale(0.95);
}
</style>
