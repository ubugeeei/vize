<script setup lang="ts">
import { ref, watch } from 'vue'
import { fetchArtSource, updateArtSource } from '../../api'
import MonacoEditor from '../MonacoEditor.vue'

const props = defineProps<{
  isOpen: boolean
  artPath: string
  artTitle: string
  tokenPaths?: string[]
}>()

const emit = defineEmits<{
  close: []
  saved: []
}>()

const source = ref('')
const loading = ref(false)
const saving = ref(false)
const error = ref<string | null>(null)

watch(() => props.isOpen, async (open) => {
  if (open && props.artPath) {
    loading.value = true
    error.value = null
    try {
      const data = await fetchArtSource(props.artPath)
      source.value = data.source
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }
})

async function handleSave() {
  saving.value = true
  error.value = null
  try {
    await updateArtSource(props.artPath, source.value)
    emit('saved')
    emit('close')
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="isOpen" class="modal-overlay" @click.self="emit('close')">
        <div class="modal-panel">
          <div class="modal-header">
            <div>
              <h2 class="modal-title">Edit Source</h2>
              <p class="modal-subtitle">{{ artTitle }}</p>
            </div>
            <button type="button" class="modal-close" @click="emit('close')">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          <div class="modal-body">
            <div v-if="loading" class="editor-loading">Loading source...</div>
            <MonacoEditor
              v-else
              v-model="source"
              language="html"
              height="500px"
              :completion-items="tokenPaths"
            />
            <p v-if="error" class="editor-error">{{ error }}</p>
          </div>

          <div class="modal-footer">
            <span class="save-hint">Cmd+S / Ctrl+S to save</span>
            <div class="modal-footer-actions">
              <button type="button" class="btn btn--secondary" @click="emit('close')">Cancel</button>
              <button type="button" class="btn btn--primary" :disabled="saving || loading" @click="handleSave">
                {{ saving ? 'Saving...' : 'Save' }}
              </button>
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
  background: rgba(0, 0, 0, 0.92);
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
  max-width: 900px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
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
  margin-bottom: 0.125rem;
}

.modal-subtitle {
  font-size: 0.75rem;
  color: var(--musea-text-muted);
  font-family: var(--musea-font-mono);
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
  flex: 1;
  overflow: hidden;
  padding: 1rem 1.5rem;
  display: flex;
  flex-direction: column;
}

.editor-loading {
  text-align: center;
  color: var(--musea-text-muted);
  padding: 3rem 0;
}

.editor-error {
  color: #ef4444;
  font-size: 0.75rem;
  margin-top: 0.5rem;
}

.modal-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1.5rem;
  border-top: 1px solid var(--musea-border);
}

.save-hint {
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
}

.modal-footer-actions {
  display: flex;
  gap: 0.5rem;
}

.btn {
  padding: 0.375rem 1rem;
  font-size: 0.8125rem;
  font-weight: 500;
  border-radius: var(--musea-radius-sm, 4px);
  cursor: pointer;
  border: 1px solid var(--musea-border);
  transition: all var(--musea-transition);
}

.btn--secondary {
  background: transparent;
  color: var(--musea-text-muted);
}

.btn--secondary:hover {
  border-color: var(--musea-text-muted);
  color: var(--musea-text);
}

.btn--primary {
  background: var(--musea-accent);
  color: #fff;
  border-color: var(--musea-accent);
}

.btn--primary:hover:not(:disabled) {
  filter: brightness(1.15);
}

.btn--primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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
