<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import type { DesignToken } from '../../api'

const props = defineProps<{
  isOpen: boolean
  mode: 'create' | 'edit'
  editPath?: string
  editToken?: DesignToken
  primitiveTokenPaths: string[]
  existingPaths: string[]
}>()

const emit = defineEmits<{
  close: []
  submit: [path: string, token: Omit<DesignToken, '$resolvedValue'>]
}>()

const tokenPath = ref('')
const tokenValue = ref<string>('')
const tokenType = ref('')
const tokenDescription = ref('')
const tier = ref<'primitive' | 'semantic'>('primitive')
const reference = ref('')
const validationError = ref<string | null>(null)

const TOKEN_TYPES = ['color', 'dimension', 'spacing', 'fontSize', 'fontWeight', 'lineHeight', 'shadow', 'borderRadius', 'opacity', 'string', 'number']

watch(() => props.isOpen, (open) => {
  if (open) {
    if (props.mode === 'edit' && props.editToken && props.editPath) {
      tokenPath.value = props.editPath
      tokenValue.value = String(props.editToken.value)
      tokenType.value = props.editToken.type ?? ''
      tokenDescription.value = props.editToken.description ?? ''
      tier.value = props.editToken.$tier ?? 'primitive'
      reference.value = props.editToken.$reference ?? ''
    } else {
      tokenPath.value = ''
      tokenValue.value = ''
      tokenType.value = ''
      tokenDescription.value = ''
      tier.value = 'primitive'
      reference.value = ''
    }
    validationError.value = null
    nextTick(() => {
      const input = document.querySelector('.token-form-path-input') as HTMLInputElement | null
      input?.focus()
    })
  }
})

const referenceOptions = computed(() => {
  if (!reference.value) return props.primitiveTokenPaths
  const q = reference.value.toLowerCase()
  return props.primitiveTokenPaths.filter(p => p.toLowerCase().includes(q))
})

const title = computed(() => props.mode === 'create' ? 'Add Token' : 'Edit Token')

function validate(): boolean {
  if (!tokenPath.value.trim()) {
    validationError.value = 'Token path is required'
    return false
  }
  if (props.mode === 'create' && props.existingPaths.includes(tokenPath.value)) {
    validationError.value = 'A token already exists at this path'
    return false
  }
  if (tier.value === 'semantic' && !reference.value.trim()) {
    validationError.value = 'Semantic tokens require a reference'
    return false
  }
  if (tier.value === 'primitive' && !tokenValue.value.trim()) {
    validationError.value = 'Token value is required'
    return false
  }
  validationError.value = null
  return true
}

function handleSubmit() {
  if (!validate()) return

  const token: Omit<DesignToken, '$resolvedValue'> = {
    value: tier.value === 'semantic' ? `{${reference.value}}` : tokenValue.value,
    $tier: tier.value,
  }
  if (tokenType.value) token.type = tokenType.value
  if (tokenDescription.value) token.description = tokenDescription.value
  if (tier.value === 'semantic') token.$reference = reference.value

  emit('submit', tokenPath.value, token)
}

function selectReference(path: string) {
  reference.value = path
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="isOpen" class="modal-overlay" @click.self="emit('close')">
        <div class="modal-content">
          <div class="modal-header">
            <h2 class="modal-title">{{ title }}</h2>
            <button type="button" class="modal-close" @click="emit('close')">
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="18" y1="6" x2="6" y2="18" />
                <line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>

          <form class="modal-form" @submit.prevent="handleSubmit">
            <div class="form-field">
              <label class="form-label">Token Path</label>
              <input
                v-model="tokenPath"
                class="form-input token-form-path-input"
                :disabled="mode === 'edit'"
                placeholder="e.g. color.primary.500"
              >
            </div>

            <div class="form-field">
              <label class="form-label">Tier</label>
              <div class="tier-radio-group">
                <label class="tier-radio" :class="{ 'tier-radio--active': tier === 'primitive' }">
                  <input v-model="tier" type="radio" value="primitive" class="tier-radio-input">
                  <span class="tier-radio-label">Primitive</span>
                </label>
                <label class="tier-radio" :class="{ 'tier-radio--active': tier === 'semantic' }">
                  <input v-model="tier" type="radio" value="semantic" class="tier-radio-input">
                  <span class="tier-radio-label">Semantic</span>
                </label>
              </div>
            </div>

            <template v-if="tier === 'primitive'">
              <div class="form-field">
                <label class="form-label">Value</label>
                <input
                  v-model="tokenValue"
                  class="form-input"
                  placeholder="e.g. #3b82f6, 16px, 400"
                >
              </div>
            </template>

            <template v-else>
              <div class="form-field">
                <label class="form-label">Reference</label>
                <input
                  v-model="reference"
                  class="form-input"
                  placeholder="e.g. color.blue.500"
                >
                <div v-if="referenceOptions.length > 0" class="reference-list">
                  <button
                    v-for="opt in referenceOptions.slice(0, 8)"
                    :key="opt"
                    type="button"
                    class="reference-option"
                    :class="{ 'reference-option--selected': opt === reference }"
                    @click="selectReference(opt)"
                  >
                    {{ opt }}
                  </button>
                </div>
              </div>
            </template>

            <div class="form-field">
              <label class="form-label">Type</label>
              <select v-model="tokenType" class="form-input form-select">
                <option value="">None</option>
                <option v-for="t in TOKEN_TYPES" :key="t" :value="t">{{ t }}</option>
              </select>
            </div>

            <div class="form-field">
              <label class="form-label">Description</label>
              <input
                v-model="tokenDescription"
                class="form-input"
                placeholder="Optional description"
              >
            </div>

            <div v-if="validationError" class="form-error">
              {{ validationError }}
            </div>

            <div class="modal-footer">
              <button type="button" class="btn btn--secondary" @click="emit('close')">Cancel</button>
              <button type="submit" class="btn btn--primary">
                {{ mode === 'create' ? 'Create' : 'Save' }}
              </button>
            </div>
          </form>
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
  background: var(--musea-overlay);
  backdrop-filter: blur(4px);
}

.modal-content {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-lg, 12px);
  width: 90%;
  max-width: 480px;
  max-height: 85vh;
  overflow-y: auto;
  padding: 1.5rem;
  box-shadow: var(--musea-shadow);
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1.5rem;
}

.modal-title {
  font-size: 1.125rem;
  font-weight: 700;
}

.modal-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: var(--musea-text-muted);
  border-radius: var(--musea-radius-sm, 4px);
  cursor: pointer;
}

.modal-close:hover {
  background: var(--musea-border);
  color: var(--musea-text);
}

.modal-form {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.form-field {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.form-label {
  font-size: 0.8125rem;
  font-weight: 600;
  color: var(--musea-text-muted);
}

.form-input {
  background: var(--musea-bg-primary, #0d0d0d);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  padding: 0.5rem 0.75rem;
  color: var(--musea-text);
  font-size: 0.8125rem;
  outline: none;
  transition: border-color var(--musea-transition);
}

.form-input:focus {
  border-color: var(--musea-accent);
}

.form-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.form-select {
  cursor: pointer;
}

.form-select option {
  background: var(--musea-bg-secondary);
  color: var(--musea-text);
}

.tier-radio-group {
  display: flex;
  gap: 0.5rem;
}

.tier-radio {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.5rem;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  cursor: pointer;
  transition: all var(--musea-transition);
}

.tier-radio--active {
  border-color: var(--musea-accent);
  background: rgba(163, 72, 40, 0.1);
}

.tier-radio-input {
  display: none;
}

.tier-radio-label {
  font-size: 0.8125rem;
  font-weight: 600;
}

.reference-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.25rem;
  max-height: 120px;
  overflow-y: auto;
  margin-top: 0.25rem;
}

.reference-option {
  font-size: 0.6875rem;
  font-family: var(--musea-font-mono);
  padding: 0.25rem 0.5rem;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm, 4px);
  background: transparent;
  color: var(--musea-text-muted);
  cursor: pointer;
  transition: all var(--musea-transition);
}

.reference-option:hover {
  border-color: var(--musea-accent);
  color: var(--musea-text);
}

.reference-option--selected {
  border-color: var(--musea-accent);
  background: rgba(163, 72, 40, 0.15);
  color: var(--musea-text);
}

.form-error {
  color: #ef4444;
  font-size: 0.8125rem;
  padding: 0.5rem;
  background: rgba(239, 68, 68, 0.1);
  border-radius: var(--musea-radius-md);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  margin-top: 0.5rem;
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

.btn--primary {
  background: var(--musea-accent);
  color: #fff;
}

.btn--primary:hover {
  filter: brightness(1.15);
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
