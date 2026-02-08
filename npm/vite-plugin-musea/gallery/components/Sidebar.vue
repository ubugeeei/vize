<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import type { ArtFileInfo } from '../../src/types.js'

const props = defineProps<{
  arts: ArtFileInfo[]
}>()

const route = useRoute()
const router = useRouter()

// Track expanded categories
const expandedCategories = ref<Set<string>>(new Set())

const categoryList = computed(() => {
  const map = new Map<string, ArtFileInfo[]>()
  for (const art of props.arts) {
    const cat = art.metadata.category || 'Components'
    if (!map.has(cat)) map.set(cat, [])
    map.get(cat)!.push(art)
  }
  // Auto-expand all categories initially
  for (const cat of map.keys()) {
    expandedCategories.value.add(cat)
  }
  return Array.from(map.entries())
})

const selectedPath = computed(() => route.params.path as string | undefined)

function toggleCategory(category: string) {
  if (expandedCategories.value.has(category)) {
    expandedCategories.value.delete(category)
  } else {
    expandedCategories.value.add(category)
  }
}

function isCategoryExpanded(category: string) {
  return expandedCategories.value.has(category)
}

function selectArt(art: ArtFileInfo) {
  router.push({ name: 'component', params: { path: art.path } })
}
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-section">
      <router-link
        :to="{ name: 'home' }"
        class="sidebar-home-link"
        :class="{ active: route.name === 'home' }"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
          <polyline points="9 22 9 12 15 12 15 22" />
        </svg>
        Home
      </router-link>

      <router-link
        :to="{ name: 'tokens' }"
        class="sidebar-home-link"
        :class="{ active: route.name === 'tokens' }"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="5" />
          <line x1="12" y1="1" x2="12" y2="3" />
          <line x1="12" y1="21" x2="12" y2="23" />
          <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
          <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
          <line x1="1" y1="12" x2="3" y2="12" />
          <line x1="21" y1="12" x2="23" y2="12" />
          <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
          <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
        </svg>
        Design Tokens
      </router-link>
    </div>

    <div
      v-for="[category, items] in categoryList"
      :key="category"
      class="sidebar-section"
    >
      <div
        class="category-header"
        :class="{ 'category-header--expanded': isCategoryExpanded(category) }"
        @click="toggleCategory(category)"
      >
        <svg class="category-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="m9 18 6-6-6-6" />
        </svg>
        <span class="category-label">{{ category }}</span>
        <span class="category-count">{{ items.length }}</span>
      </div>
      <ul v-show="isCategoryExpanded(category)" class="art-list">
        <li
          v-for="art in items"
          :key="art.path"
          class="art-item"
          :class="{ active: selectedPath === art.path }"
          @click="selectArt(art)"
        >
          <span class="art-name">{{ art.metadata.title }}</span>
          <span class="art-variant-count">{{ art.variants.length }}</span>
        </li>
      </ul>
    </div>

    <div v-if="arts.length === 0" class="sidebar-empty">
      No components found
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  background: var(--musea-bg-secondary);
  border-right: 1px solid var(--musea-border);
  overflow-y: auto;
  overflow-x: hidden;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.sidebar-section {
  padding: 0.25rem 0.375rem;
  flex-shrink: 0;
}

.sidebar-section + .sidebar-section {
  padding-top: 0;
}

.sidebar-home-link {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.375rem;
  border-radius: 2px;
  font-size: 0.625rem;
  color: var(--musea-text-secondary);
  cursor: pointer;
  transition: all var(--musea-transition);
  text-decoration: none;
}

.sidebar-home-link svg {
  width: 10px;
  height: 10px;
}

.sidebar-home-link:hover {
  background: var(--musea-bg-tertiary);
  color: var(--musea-text);
}

.sidebar-home-link.active {
  background: var(--musea-accent-subtle);
  color: var(--musea-accent-hover);
}

.category-header {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.375rem;
  font-size: 0.5625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--musea-text-muted);
  cursor: pointer;
  user-select: none;
  border-radius: 2px;
  transition: background var(--musea-transition);
}

.category-header:hover {
  background: var(--musea-bg-tertiary);
}

.category-icon {
  width: 10px;
  height: 10px;
  transition: transform 0.15s ease;
  flex-shrink: 0;
}

.category-header--expanded .category-icon {
  transform: rotate(90deg);
}

.category-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.category-count {
  flex-shrink: 0;
  background: var(--musea-bg-tertiary);
  padding: 0 0.25rem;
  border-radius: 2px;
  font-size: 0.5rem;
}

.art-list {
  list-style: none;
  margin: 0;
  padding: 0;
}

.art-item {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.1875rem 0.375rem 0.1875rem 1rem;
  border-radius: 2px;
  cursor: pointer;
  font-size: 0.625rem;
  color: var(--musea-text-secondary);
  transition: all var(--musea-transition);
  position: relative;
}

.art-item::before {
  content: '';
  position: absolute;
  left: 0.5rem;
  top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: var(--musea-border);
  transition: background var(--musea-transition);
}

.art-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.art-item:hover {
  background: var(--musea-bg-tertiary);
  color: var(--musea-text);
}

.art-item:hover::before {
  background: var(--musea-text-muted);
}

.art-item.active {
  background: var(--musea-accent-subtle);
  color: var(--musea-accent-hover);
}

.art-item.active::before {
  background: var(--musea-accent);
}

.art-variant-count {
  flex-shrink: 0;
  font-size: 0.5rem;
  color: var(--musea-text-muted);
  background: var(--musea-bg-tertiary);
  padding: 0 0.25rem;
  border-radius: 6px;
  opacity: 0.7;
  transition: opacity var(--musea-transition);
}

.art-item:hover .art-variant-count {
  opacity: 1;
}

.art-item.active .art-variant-count {
  background: var(--musea-accent);
  color: white;
  opacity: 1;
}

.sidebar-empty {
  padding: 1rem 0.5rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.625rem;
}
</style>
