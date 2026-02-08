<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useArts } from '../composables/useArts'
import { useSearch } from '../composables/useSearch'
import SearchBar from './SearchBar.vue'
import Sidebar from './Sidebar.vue'
import SearchModal from './SearchModal.vue'

const router = useRouter()
const { arts, load } = useArts()
const { query, results } = useSearch(arts)

const searchModalOpen = ref(false)

// Global keyboard shortcut for Cmd+K / Ctrl+K
const handleKeydown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault()
    searchModalOpen.value = !searchModalOpen.value
  }
}

onMounted(() => {
  load()
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
})

const handleSearchSelect = (art: { path: string }, variantName?: string) => {
  router.push({ name: 'component', params: { path: art.path } })
}
</script>

<template>
  <div class="gallery-layout">
    <header class="header">
      <div class="header-left">
        <router-link to="/" class="logo">
          <svg class="logo-svg" width="32" height="32" viewBox="0 0 200 200" fill="none">
            <defs>
              <linearGradient id="metal-grad" x1="0%" y1="0%" x2="100%" y2="20%">
                <stop offset="0%" stop-color="#f0f2f5" />
                <stop offset="50%" stop-color="#9ca3b0" />
                <stop offset="100%" stop-color="#e07048" />
              </linearGradient>
              <linearGradient id="metal-grad-dark" x1="0%" y1="0%" x2="100%" y2="30%">
                <stop offset="0%" stop-color="#d0d4dc" />
                <stop offset="60%" stop-color="#6b7280" />
                <stop offset="100%" stop-color="#c45530" />
              </linearGradient>
            </defs>
            <g transform="translate(40, 40)">
              <g transform="skewX(-12)">
                <path d="M 100 0 L 60 120 L 105 30 L 100 0 Z" fill="url(#metal-grad-dark)" stroke="#4b5563" stroke-width="0.5" />
                <path d="M 30 0 L 60 120 L 80 20 L 30 0 Z" fill="url(#metal-grad)" stroke-width="0.5" stroke-opacity="0.4" />
              </g>
            </g>
            <g transform="translate(110, 120)">
              <line x1="5" y1="10" x2="5" y2="50" stroke="#e07048" stroke-width="3" stroke-linecap="round" />
              <line x1="60" y1="10" x2="60" y2="50" stroke="#e07048" stroke-width="3" stroke-linecap="round" />
              <path d="M 0 10 L 32.5 0 L 65 10" fill="none" stroke="#e07048" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
              <rect x="15" y="18" width="14" height="12" rx="1" fill="none" stroke="#e07048" stroke-width="1.5" opacity="0.7" />
              <rect x="36" y="18" width="14" height="12" rx="1" fill="none" stroke="#e07048" stroke-width="1.5" opacity="0.7" />
              <rect x="23" y="35" width="18" height="12" rx="1" fill="none" stroke="#e07048" stroke-width="1.5" opacity="0.6" />
            </g>
          </svg>
          Musea
        </router-link>
        <span class="header-subtitle">Component Gallery</span>
      </div>

      <div class="header-center">
        <button class="search-trigger" @click="searchModalOpen = true">
          <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="8" />
            <line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <span>Search components...</span>
          <kbd>⌘K</kbd>
        </button>
      </div>
    </header>

    <main class="main">
      <!-- Sidebar -->
      <Sidebar :arts="results" />

      <!-- Main Content -->
      <section class="content">
        <router-view />
      </section>
    </main>

    <!-- Search Modal -->
    <SearchModal
      :arts="arts"
      :is-open="searchModalOpen"
      @close="searchModalOpen = false"
      @select="handleSearchSelect"
    />
  </div>
</template>

<style scoped>
.gallery-layout {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.header {
  background: var(--musea-bg-secondary);
  border-bottom: 1px solid var(--musea-border);
  padding: 0 0.75rem;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
  position: sticky;
  top: 0;
  z-index: 100;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.header-center {
  flex: 1;
  display: flex;
  justify-content: center;
  max-width: 300px;
}

.logo {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.75rem;
  font-weight: 700;
  color: var(--musea-accent);
  text-decoration: none;
}

.logo-svg {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
}

.header-subtitle {
  color: var(--musea-text-muted);
  font-size: 0.625rem;
  font-weight: 500;
  padding-left: 0.75rem;
  border-left: 1px solid var(--musea-border);
}

.search-trigger {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  width: 100%;
  padding: 0.25rem 0.5rem;
  background: var(--musea-bg-tertiary);
  border: 1px solid var(--musea-border);
  border-radius: 4px;
  color: var(--musea-text-muted);
  font-size: 0.625rem;
  cursor: pointer;
  transition: all 0.15s;
}

.search-trigger:hover {
  border-color: var(--musea-accent);
  color: var(--musea-text-secondary);
}

.search-icon {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
}

.search-trigger span {
  flex: 1;
  text-align: left;
}

.search-trigger kbd {
  padding: 0 0.25rem;
  background: var(--musea-bg-primary);
  border: 1px solid var(--musea-border);
  border-radius: 2px;
  font-size: 0.5625rem;
  font-family: inherit;
}

.main {
  display: grid;
  grid-template-columns: var(--musea-sidebar-width) 1fr;
  flex: 1;
  overflow: hidden;
  height: calc(100vh - 32px);
}

.main > :first-child {
  height: 100%;
  max-height: 100%;
  overflow-y: auto;
}

.content {
  background: var(--musea-bg-primary);
  overflow-y: auto;
  height: 100%;
}

@media (max-width: 768px) {
  .main {
    grid-template-columns: 1fr !important;
  }
  .main > :first-child {
    display: none;
  }
  .header-subtitle {
    display: none;
  }
  .header-center {
    display: none;
  }
}
</style>
