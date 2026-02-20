<script setup lang="ts">
import { ref, watch } from 'vue'
import type { DesignToken } from '../../api'

const props = defineProps<{
  isOpen: boolean
  tokenPath: string
  token?: DesignToken
  dependents?: string[]
}>()

const emit = defineEmits<{
  close: []
  confirm: []
}>()

const confirming = ref(false)

watch(() => props.isOpen, (open) => {
  if (open) confirming.value = false
})
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="isOpen" class="modal-overlay" @click.self="emit('close')">
        <div class="modal-content">
          <h3 class="modal-title">Delete Token</h3>

          <p class="delete-message">
            Are you sure you want to delete <code class="token-path">{{ tokenPath }}</code>?
          </p>

          <div v-if="dependents && dependents.length > 0" class="dependents-warning">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
              <line x1="12" y1="9" x2="12" y2="13" />
              <line x1="12" y1="17" x2="12.01" y2="17" />
            </svg>
            <div>
              <p class="warning-text">The following semantic tokens reference this token:</p>
              <ul class="dependents-list">
                <li v-for="dep in dependents" :key="dep" class="dependent-item">
                  <code>{{ dep }}</code>
                </li>
              </ul>
              <p class="warning-note">Deleting this token will leave these references unresolved.</p>
            </div>
          </div>

          <div class="modal-footer">
            <button type="button" class="btn btn--secondary" @click="emit('close')">Cancel</button>
            <button type="button" class="btn btn--danger" @click="emit('confirm')">
              Delete
            </button>
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
  z-index: 1000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.92);
  backdrop-filter: blur(4px);
}

.modal-content {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-lg, 12px);
  width: 90%;
  max-width: 420px;
  padding: 1.5rem;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
}

.modal-title {
  font-size: 1.125rem;
  font-weight: 700;
  margin-bottom: 1rem;
}

.delete-message {
  font-size: 0.875rem;
  color: var(--musea-text);
  margin-bottom: 1rem;
}

.token-path {
  font-family: var(--musea-font-mono);
  font-size: 0.8125rem;
  background: var(--musea-border);
  padding: 0.125rem 0.375rem;
  border-radius: var(--musea-radius-sm, 4px);
}

.dependents-warning {
  display: flex;
  gap: 0.75rem;
  padding: 0.75rem;
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.3);
  border-radius: var(--musea-radius-md);
  margin-bottom: 1rem;
  color: #fbbf24;
}

.dependents-warning svg {
  flex-shrink: 0;
  margin-top: 0.125rem;
}

.warning-text {
  font-size: 0.8125rem;
  margin-bottom: 0.375rem;
}

.dependents-list {
  list-style: none;
  padding: 0;
  margin-bottom: 0.375rem;
}

.dependent-item {
  font-size: 0.75rem;
  margin-bottom: 0.125rem;
}

.dependent-item code {
  font-family: var(--musea-font-mono);
}

.warning-note {
  font-size: 0.75rem;
  opacity: 0.8;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
}

.btn {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: var(--musea-radius-md);
  font-size: 0.8125rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.btn--secondary {
  background: var(--musea-border);
  color: var(--musea-text);
}

.btn--secondary:hover {
  background: var(--musea-text-muted);
}

.btn--danger {
  background: #ef4444;
  color: #fff;
}

.btn--danger:hover {
  background: #dc2626;
}

/* Transition */
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-active .modal-content,
.modal-leave-active .modal-content {
  transition: transform 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal-content {
  transform: scale(0.95);
}

.modal-leave-to .modal-content {
  transform: scale(0.95);
}
</style>
