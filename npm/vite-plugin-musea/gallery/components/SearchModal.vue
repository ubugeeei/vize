<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useRouter } from 'vue-router'
import { mdiMagnify, mdiHistory, mdiPalette, mdiDiamond, mdiHome, mdiCheckCircleOutline, mdiNavigationOutline } from '@mdi/js'
import type { ArtFileInfo } from '../../src/types.js'
import MdiIcon from './MdiIcon.vue'

const props = defineProps<{
  arts: ArtFileInfo[]
  isOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'select', art: ArtFileInfo, variantName?: string): void
}>()

const router = useRouter()

interface NavItem {
  name: string
  route: string
  icon: string
}

const navItems: NavItem[] = [
  { name: 'Home', route: '/', icon: mdiHome },
  { name: 'Design Tokens', route: '/tokens', icon: mdiPalette },
  { name: 'Test Summary', route: '/tests', icon: mdiCheckCircleOutline },
]

const searchInput = ref<HTMLInputElement | null>(null)
const query = ref('')
const selectedIndex = ref(0)
const searchHistory = ref<string[]>([])

// Load search history from localStorage
onMounted(() => {
  const saved = localStorage.getItem('musea-search-history')
  if (saved) {
    try {
      searchHistory.value = JSON.parse(saved)
    } catch {
      // ignore
    }
  }
})

// Save search history
const saveToHistory = (term: string) => {
  if (!term.trim()) return
  const history = searchHistory.value.filter(h => h !== term)
  history.unshift(term)
  searchHistory.value = history.slice(0, 10)
  localStorage.setItem('musea-search-history', JSON.stringify(searchHistory.value))
}

interface SearchResult {
  type: 'component'
  art: ArtFileInfo
  matchType: 'title' | 'category' | 'tags' | 'variant' | 'description'
  variantName?: string
  score: number
}

interface NavSearchResult {
  type: 'nav'
  nav: NavItem
  score: number
}

type AnyResult = SearchResult | NavSearchResult

// Fuzzy search with scoring
const results = computed((): AnyResult[] => {
  const q = query.value.toLowerCase().trim()
  if (!q) {
    return []
  }

  const scored: AnyResult[] = []

  // Search navigation items
  for (const nav of navItems) {
    if (nav.name.toLowerCase().includes(q)) {
      scored.push({
        type: 'nav',
        nav,
        score: nav.name.toLowerCase().startsWith(q) ? 110 : 90,
      })
    }
  }

  for (const art of props.arts) {
    const title = art.metadata.title.toLowerCase()
    const category = (art.metadata.category ?? '').toLowerCase()
    const description = (art.metadata.description ?? '').toLowerCase()
    const tags = art.metadata.tags.map(t => t.toLowerCase())

    // Title match (highest priority)
    if (title.includes(q)) {
      scored.push({
        type: 'component',
        art,
        matchType: 'title',
        score: title.startsWith(q) ? 100 : 80,
      })
      continue
    }

    // Category match
    if (category.includes(q)) {
      scored.push({
        type: 'component',
        art,
        matchType: 'category',
        score: 60,
      })
      continue
    }

    // Tag match
    const matchedTag = tags.find(t => t.includes(q))
    if (matchedTag) {
      scored.push({
        type: 'component',
        art,
        matchType: 'tags',
        score: 50,
      })
      continue
    }

    // Variant match
    const matchedVariant = art.variants.find(v => v.name.toLowerCase().includes(q))
    if (matchedVariant) {
      scored.push({
        type: 'component',
        art,
        matchType: 'variant',
        variantName: matchedVariant.name,
        score: 40,
      })
      continue
    }

    // Description match (lowest priority)
    if (description.includes(q)) {
      scored.push({
        type: 'component',
        art,
        matchType: 'description',
        score: 20,
      })
    }
  }

  return scored.sort((a, b) => b.score - a.score).slice(0, 10)
})

// Reset selection when results change
watch(results, () => {
  selectedIndex.value = 0
})

// Focus input when modal opens
watch(() => props.isOpen, (open) => {
  if (open) {
    query.value = ''
    selectedIndex.value = 0
    nextTick(() => {
      searchInput.value?.focus()
    })
  }
})

const handleKeydown = (e: KeyboardEvent) => {
  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault()
      selectedIndex.value = Math.min(selectedIndex.value + 1, results.value.length - 1)
      break
    case 'ArrowUp':
      e.preventDefault()
      selectedIndex.value = Math.max(selectedIndex.value - 1, 0)
      break
    case 'Enter':
      e.preventDefault()
      if (results.value[selectedIndex.value]) {
        selectResult(results.value[selectedIndex.value])
      }
      break
    case 'Escape':
      e.preventDefault()
      emit('close')
      break
  }
}

const selectResult = (result: AnyResult) => {
  saveToHistory(query.value)
  if (result.type === 'nav') {
    router.push(result.nav.route)
    emit('close')
  } else {
    emit('select', result.art, result.variantName)
    emit('close')
  }
}

const selectFromHistory = (term: string) => {
  query.value = term
  searchInput.value?.focus()
}

const clearHistory = () => {
  searchHistory.value = []
  localStorage.removeItem('musea-search-history')
}

// Global keyboard listener for Cmd+K / Ctrl+K
const handleGlobalKeydown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault()
    if (!props.isOpen) {
      // This should trigger parent to open
      // But handled externally
    } else {
      emit('close')
    }
  }
}

onMounted(() => {
  document.addEventListener('keydown', handleGlobalKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleGlobalKeydown)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="isOpen" class="search-modal-overlay" @click.self="emit('close')">
        <div class="search-modal" @keydown="handleKeydown">
          <!-- Search Input -->
          <div class="search-input-wrapper">
            <MdiIcon class="search-icon" :path="mdiMagnify" :size="20" />
            <input
              ref="searchInput"
              v-model="query"
              type="text"
              class="search-input"
              placeholder="Search components, variants, tags..."
              autocomplete="off"
            />
            <kbd class="search-shortcut">ESC</kbd>
          </div>

          <!-- Results -->
          <div class="search-results">
            <template v-if="results.length > 0">
              <div
                v-for="(result, index) in results"
                :key="result.type === 'nav' ? `nav-${result.nav.route}` : `${result.art.path}-${result.variantName || ''}`"
                :class="['search-result', { 'search-result--selected': index === selectedIndex }]"
                @click="selectResult(result)"
                @mouseenter="selectedIndex = index"
              >
                <template v-if="result.type === 'nav'">
                  <div class="result-icon">
                    <MdiIcon :path="result.nav.icon" :size="16" />
                  </div>
                  <div class="result-content">
                    <div class="result-title">{{ result.nav.name }}</div>
                    <div class="result-meta">
                      <span class="result-match-type">page</span>
                    </div>
                  </div>
                </template>
                <template v-else>
                  <div class="result-icon">
                    <MdiIcon v-if="result.matchType === 'variant'" :path="mdiDiamond" :size="16" />
                    <MdiIcon v-else :path="mdiPalette" :size="16" />
                  </div>
                  <div class="result-content">
                    <div class="result-title">
                      {{ result.art.metadata.title }}
                      <span v-if="result.variantName" class="result-variant">
                        / {{ result.variantName }}
                      </span>
                    </div>
                    <div class="result-meta">
                      <span v-if="result.art.metadata.category" class="result-category">
                        {{ result.art.metadata.category }}
                      </span>
                      <span class="result-match-type">{{ result.matchType }}</span>
                    </div>
                  </div>
                </template>
                <kbd class="result-shortcut">↵</kbd>
              </div>
            </template>

            <!-- Empty state with history -->
            <template v-else-if="!query && searchHistory.length > 0">
              <div class="search-history-header">
                <span>Recent Searches</span>
                <button type="button" class="history-clear" @click="clearHistory">Clear</button>
              </div>
              <div
                v-for="term in searchHistory"
                :key="term"
                class="search-history-item"
                @click="selectFromHistory(term)"
              >
                <MdiIcon class="history-icon" :path="mdiHistory" :size="14" />
                {{ term }}
              </div>
            </template>

            <!-- No results -->
            <div v-else-if="query" class="search-empty">
              No results for "{{ query }}"
            </div>

            <!-- Initial state -->
            <div v-else class="search-hint">
              Start typing to search components
            </div>
          </div>

          <!-- Footer -->
          <div class="search-footer">
            <div class="search-footer-item">
              <kbd>↑</kbd><kbd>↓</kbd> to navigate
            </div>
            <div class="search-footer-item">
              <kbd>↵</kbd> to select
            </div>
            <div class="search-footer-item">
              <kbd>esc</kbd> to close
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.search-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.92);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 15vh;
  z-index: 1000;
}

.search-modal {
  width: 100%;
  max-width: 560px;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: 12px;
  box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.search-input-wrapper {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem;
  border-bottom: 1px solid var(--musea-border);
}

.search-icon {
  width: 20px;
  height: 20px;
  color: var(--musea-text-muted);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  background: transparent;
  border: none;
  font-size: 1rem;
  color: var(--musea-text);
  outline: none;
}

.search-input::placeholder {
  color: var(--musea-text-muted);
}

.search-shortcut {
  padding: 0.25rem 0.5rem;
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: 4px;
  font-size: 0.6875rem;
  font-family: inherit;
  color: var(--musea-text-muted);
}

.search-results {
  max-height: 400px;
  overflow-y: auto;
  padding: 0.5rem;
}

.search-result {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.75rem;
  border-radius: 8px;
  cursor: pointer;
  transition: background-color 0.1s;
}

.search-result:hover,
.search-result--selected {
  background: var(--musea-bg-tertiary);
}

.result-icon {
  font-size: 1rem;
  width: 24px;
  text-align: center;
  flex-shrink: 0;
}

.result-content {
  flex: 1;
  min-width: 0;
}

.result-title {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--musea-text);
}

.result-variant {
  color: var(--musea-accent);
}

.result-meta {
  display: flex;
  gap: 0.5rem;
  margin-top: 0.25rem;
}

.result-category {
  font-size: 0.75rem;
  color: var(--musea-text-muted);
}

.result-match-type {
  font-size: 0.625rem;
  padding: 0.0625rem 0.375rem;
  background: var(--musea-accent-subtle);
  color: var(--musea-accent);
  border-radius: 3px;
  text-transform: uppercase;
}

.result-shortcut {
  padding: 0.125rem 0.375rem;
  background: var(--musea-bg-primary);
  border: 1px solid var(--musea-border);
  border-radius: 3px;
  font-size: 0.625rem;
  font-family: inherit;
  color: var(--musea-text-muted);
  opacity: 0;
  transition: opacity 0.1s;
}

.search-result--selected .result-shortcut {
  opacity: 1;
}

.search-history-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--musea-text-muted);
}

.history-clear {
  background: transparent;
  border: none;
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
  cursor: pointer;
}

.history-clear:hover {
  color: var(--musea-text);
}

.search-history-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.625rem 0.75rem;
  border-radius: 6px;
  font-size: 0.875rem;
  color: var(--musea-text-secondary);
  cursor: pointer;
  transition: background-color 0.1s;
}

.search-history-item:hover {
  background: var(--musea-bg-tertiary);
  color: var(--musea-text);
}

.history-icon {
  width: 14px;
  height: 14px;
  color: var(--musea-text-muted);
}

.search-empty,
.search-hint {
  padding: 2rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.875rem;
}

.search-footer {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1.5rem;
  padding: 0.75rem;
  border-top: 1px solid var(--musea-border);
  background: var(--musea-bg-tertiary);
}

.search-footer-item {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
}

.search-footer-item kbd {
  padding: 0.125rem 0.375rem;
  background: var(--musea-bg-primary);
  border: 1px solid var(--musea-border);
  border-radius: 3px;
  font-size: 0.625rem;
  font-family: inherit;
  min-width: 18px;
  text-align: center;
}

/* Transition */
.modal-enter-active,
.modal-leave-active {
  transition: all 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .search-modal,
.modal-leave-to .search-modal {
  transform: scale(0.95) translateY(-20px);
}
</style>
