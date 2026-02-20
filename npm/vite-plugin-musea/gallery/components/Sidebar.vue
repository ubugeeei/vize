<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { mdiHome, mdiPalette, mdiCheckCircleOutline, mdiChevronRight } from '@mdi/js'
import type { ArtFileInfo } from '../../src/types.js'
import MdiIcon from './MdiIcon.vue'

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
        <MdiIcon :path="mdiHome" :size="16" />
        Home
      </router-link>

      <router-link
        :to="{ name: 'tokens' }"
        class="sidebar-home-link"
        :class="{ active: route.name === 'tokens' }"
      >
        <MdiIcon :path="mdiPalette" :size="16" />
        Design Tokens
      </router-link>

      <router-link
        :to="{ name: 'tests' }"
        class="sidebar-home-link"
        :class="{ active: route.name === 'tests' }"
      >
        <MdiIcon :path="mdiCheckCircleOutline" :size="16" />
        Test Summary
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
        <MdiIcon class="category-icon" :path="mdiChevronRight" :size="16" />
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
  overflow-y: auto;
  overflow-x: hidden;
}

.sidebar-section {
  padding: 0.5rem 0.75rem;
}

.sidebar-section + .sidebar-section {
  padding-top: 0;
}

.sidebar-home-link {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.5rem 0.75rem;
  border-radius: var(--musea-radius-sm);
  font-size: 0.8125rem;
  color: var(--musea-text-secondary);
  cursor: pointer;
  transition: all var(--musea-transition);
  text-decoration: none;
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
  gap: 0.5rem;
  padding: 0.625rem 0.75rem;
  font-size: 0.6875rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--musea-text-muted);
  cursor: pointer;
  user-select: none;
  border-radius: var(--musea-radius-sm);
  transition: background var(--musea-transition);
}

.category-header:hover {
  background: var(--musea-bg-tertiary);
}

.category-icon {
  width: 16px;
  height: 16px;
  transition: transform var(--musea-transition);
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
  margin-left: auto;
  background: var(--musea-bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-size: 0.625rem;
}

.art-list {
  list-style: none;
  margin: 0;
  margin-top: 0.25rem;
  padding: 0;
}

.art-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.375rem 0.75rem 0.375rem 2.25rem;
  border-radius: var(--musea-radius-sm);
  cursor: pointer;
  font-size: 0.8125rem;
  color: var(--musea-text-secondary);
  transition: all var(--musea-transition);
  position: relative;
}

.art-item::before {
  content: '';
  position: absolute;
  left: 1.25rem;
  top: 50%;
  transform: translateY(-50%);
  width: 5px;
  height: 5px;
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
  margin-left: auto;
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
  opacity: 0;
  transition: opacity var(--musea-transition);
}

.art-item:hover .art-variant-count {
  opacity: 1;
}

.sidebar-empty {
  padding: 2rem 1rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.8125rem;
}
</style>
