<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { DesignToken, TokenUsageEntry } from '../api'
import { useTokens } from '../composables/useTokens'
import { useTokenUsage } from '../composables/useTokenUsage'
import TokenCategorySection from '../components/tokens/TokenCategorySection.vue'
import TokenFormModal from '../components/tokens/TokenFormModal.vue'
import TokenDeleteConfirm from '../components/tokens/TokenDeleteConfirm.vue'
import TokenUsageModal from '../components/tokens/TokenUsageModal.vue'
import SourceEditModal from '../components/tokens/SourceEditModal.vue'

const {
  loading,
  error,
  activeTab,
  filter,
  filteredCategories,
  primitiveTokenPaths,
  tokenMap,
  meta,
  load,
  addToken,
  editToken,
  removeToken,
} = useTokens()

const {
  usageMap,
  load: loadUsage,
  reload: reloadUsage,
  getUsage,
} = useTokenUsage()

// Usage modal state
const showUsageModal = ref(false)
const usageTokenPath = ref('')
const usageTokenData = ref<DesignToken | undefined>()
const usageEntries = ref<TokenUsageEntry[]>([])

// Source editor modal state
const showSourceEditor = ref(false)
const sourceEditorArtPath = ref('')
const sourceEditorArtTitle = ref('')

// Modal state
const showFormModal = ref(false)
const formMode = ref<'create' | 'edit'>('create')
const editPath = ref('')
const editTokenData = ref<DesignToken | undefined>()

const showDeleteConfirm = ref(false)
const deletePath = ref('')
const deleteTokenData = ref<DesignToken | undefined>()
const deleteDependents = ref<string[]>([])

const existingPaths = computed(() => Object.keys(tokenMap.value))

const tabs = [
  { key: 'all' as const, label: 'All' },
  { key: 'primitive' as const, label: 'Primitive' },
  { key: 'semantic' as const, label: 'Semantic' },
] as const

onMounted(() => {
  load()
  loadUsage()
})

function openCreateModal() {
  formMode.value = 'create'
  editPath.value = ''
  editTokenData.value = undefined
  showFormModal.value = true
}

function openEditModal(path: string, token: DesignToken) {
  formMode.value = 'edit'
  editPath.value = path
  editTokenData.value = token
  showFormModal.value = true
}

function openDeleteConfirm(path: string, token: DesignToken) {
  deletePath.value = path
  deleteTokenData.value = token
  // Find dependents from tokenMap
  const deps: string[] = []
  for (const [p, t] of Object.entries(tokenMap.value)) {
    if (t.$reference === path) {
      deps.push(p)
    }
  }
  deleteDependents.value = deps
  showDeleteConfirm.value = true
}

async function handleFormSubmit(path: string, token: Omit<DesignToken, '$resolvedValue'>) {
  try {
    if (formMode.value === 'create') {
      await addToken(path, token)
    } else {
      await editToken(path, token)
    }
    showFormModal.value = false
  } catch (e) {
    // Error is handled inside the modal via validation
    console.error('[musea] Token save error:', e)
  }
}

async function handleDeleteConfirm() {
  try {
    await removeToken(deletePath.value)
    showDeleteConfirm.value = false
  } catch (e) {
    console.error('[musea] Token delete error:', e)
  }
}

function openUsageModal(tokenPath: string) {
  usageTokenPath.value = tokenPath
  usageTokenData.value = tokenMap.value[tokenPath]
  usageEntries.value = getUsage(tokenPath)
  showUsageModal.value = true
}

function openSourceEditor(artPath: string) {
  // Find art title from usage entries
  const entry = usageEntries.value.find(e => e.artPath === artPath)
  sourceEditorArtPath.value = artPath
  sourceEditorArtTitle.value = entry?.artTitle ?? artPath.split('/').pop() ?? artPath
  showUsageModal.value = false
  showSourceEditor.value = true
}

async function handleSourceSaved() {
  await reloadUsage()
}
</script>

<template>
  <div class="tokens-view">
    <div class="tokens-header">
      <div class="tokens-header-top">
        <div>
          <h1 class="tokens-title">Design Tokens</h1>
          <p class="tokens-description">
            Browse and manage design tokens from your Style Dictionary configuration
          </p>
        </div>
        <button type="button" class="add-token-btn" @click="openCreateModal">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="12" y1="5" x2="12" y2="19" />
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
          Add Token
        </button>
      </div>

      <div class="tokens-toolbar">
        <div class="tab-bar">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            type="button"
            class="tab-btn"
            :class="{ 'tab-btn--active': activeTab === tab.key }"
            @click="activeTab = tab.key"
          >
            {{ tab.label }}
            <span v-if="tab.key === 'all' && meta.tokenCount" class="tab-count">{{ meta.tokenCount }}</span>
            <span v-else-if="tab.key === 'primitive' && meta.primitiveCount" class="tab-count">{{ meta.primitiveCount }}</span>
            <span v-else-if="tab.key === 'semantic' && meta.semanticCount" class="tab-count">{{ meta.semanticCount }}</span>
          </button>
        </div>

        <input
          v-model="filter"
          type="text"
          class="tokens-filter"
          placeholder="Filter tokens..."
        >
      </div>
    </div>

    <div v-if="loading" class="tokens-loading">
      <div class="loading-spinner" />
      Loading tokens...
    </div>

    <div v-else-if="error" class="tokens-empty">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <circle cx="12" cy="12" r="5" />
        <line x1="12" y1="1" x2="12" y2="3" />
        <line x1="12" y1="21" x2="12" y2="23" />
        <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
        <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
        <line x1="1" y1="12" x2="3" y2="12" />
        <line x1="21" y1="12" x2="23" y2="12" />
      </svg>
      <p>{{ error }}</p>
    </div>

    <div v-else-if="filteredCategories.length === 0" class="tokens-empty">
      <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <circle cx="12" cy="12" r="5" />
        <line x1="12" y1="1" x2="12" y2="3" />
        <line x1="12" y1="21" x2="12" y2="23" />
        <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
        <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
        <line x1="1" y1="12" x2="3" y2="12" />
        <line x1="21" y1="12" x2="23" y2="12" />
      </svg>
      <p v-if="filter">No tokens match your search.</p>
      <p v-else>No design tokens configured. Add a Style Dictionary config to see tokens here.</p>
    </div>

    <template v-else>
      <TokenCategorySection
        v-for="cat in filteredCategories"
        :key="cat.name"
        :category="cat"
        :level="2"
        :usage-map="usageMap"
        @edit="openEditModal"
        @delete="openDeleteConfirm"
        @show-usage="openUsageModal"
      />
    </template>

    <TokenFormModal
      :is-open="showFormModal"
      :mode="formMode"
      :edit-path="editPath"
      :edit-token="editTokenData"
      :primitive-token-paths="primitiveTokenPaths"
      :existing-paths="existingPaths"
      @close="showFormModal = false"
      @submit="handleFormSubmit"
    />

    <TokenDeleteConfirm
      :is-open="showDeleteConfirm"
      :token-path="deletePath"
      :token="deleteTokenData"
      :dependents="deleteDependents"
      @close="showDeleteConfirm = false"
      @confirm="handleDeleteConfirm"
    />

    <TokenUsageModal
      :is-open="showUsageModal"
      :token-path="usageTokenPath"
      :token="usageTokenData"
      :usages="usageEntries"
      @close="showUsageModal = false"
      @edit-source="openSourceEditor"
    />

    <SourceEditModal
      :is-open="showSourceEditor"
      :art-path="sourceEditorArtPath"
      :art-title="sourceEditorArtTitle"
      :token-paths="existingPaths"
      @close="showSourceEditor = false"
      @saved="handleSourceSaved"
    />
  </div>
</template>

<style scoped>
.tokens-view {
  max-width: 1400px;
  margin: 0 auto;
  padding: 2rem;
}

.tokens-header {
  margin-bottom: 2rem;
}

.tokens-header-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  margin-bottom: 1rem;
}

.tokens-title {
  font-size: 1.5rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
}

.tokens-description {
  color: var(--musea-text-muted);
  font-size: 0.9375rem;
}

.add-token-btn {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.5rem 1rem;
  background: var(--musea-accent);
  color: #fff;
  border: none;
  border-radius: var(--musea-radius-md);
  font-size: 0.8125rem;
  font-weight: 600;
  cursor: pointer;
  transition: filter var(--musea-transition);
  white-space: nowrap;
}

.add-token-btn:hover {
  filter: brightness(1.15);
}

.tokens-toolbar {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.tab-bar {
  display: flex;
  gap: 0.25rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  padding: 0.25rem;
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.375rem 0.75rem;
  border: none;
  background: transparent;
  color: var(--musea-text-muted);
  font-size: 0.8125rem;
  font-weight: 500;
  border-radius: calc(var(--musea-radius-md) - 2px);
  cursor: pointer;
  transition: all var(--musea-transition);
}

.tab-btn:hover {
  color: var(--musea-text);
}

.tab-btn--active {
  background: var(--musea-border);
  color: var(--musea-text);
  font-weight: 600;
}

.tab-count {
  font-size: 0.6875rem;
  background: var(--musea-border);
  padding: 0 0.375rem;
  border-radius: 9999px;
  color: var(--musea-text-muted);
}

.tab-btn--active .tab-count {
  background: var(--musea-text-muted);
  color: var(--musea-bg-secondary);
}

.tokens-filter {
  flex: 1;
  min-width: 200px;
  max-width: 400px;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  padding: 0.5rem 0.75rem;
  color: var(--musea-text);
  font-size: 0.8125rem;
  outline: none;
  transition: border-color var(--musea-transition);
}

.tokens-filter:focus {
  border-color: var(--musea-accent);
}

.tokens-loading {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  justify-content: center;
  min-height: 200px;
  color: var(--musea-text-muted);
}

.loading-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--musea-border);
  border-top-color: var(--musea-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.tokens-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 300px;
  text-align: center;
  color: var(--musea-text-muted);
  gap: 1rem;
}
</style>
