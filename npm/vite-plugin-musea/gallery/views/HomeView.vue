<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useArts } from '../composables/useArts'
import StatusBadge from '../components/StatusBadge.vue'

const { arts, categories } = useArts()
const router = useRouter()

const categoryList = computed(() => Array.from(categories.value.entries()))

const stats = computed(() => ({
  total: arts.value.length,
  variants: arts.value.reduce((sum, a) => sum + a.variants.length, 0),
  categories: categories.value.size,
}))

function goToArt(path: string) {
  router.push({ name: 'component', params: { path } })
}
</script>

<template>
  <div class="home">
    <div class="home-header">
      <h1 class="home-title">Component Gallery</h1>
      <p class="home-description">
        Browse, preview, and interact with your components
      </p>
      <div class="home-stats">
        <div class="home-stat">
          <span class="home-stat-value">{{ stats.total }}</span>
          <span class="home-stat-label">Components</span>
        </div>
        <div class="home-stat">
          <span class="home-stat-value">{{ stats.variants }}</span>
          <span class="home-stat-label">Variants</span>
        </div>
        <div class="home-stat">
          <span class="home-stat-value">{{ stats.categories }}</span>
          <span class="home-stat-label">Categories</span>
        </div>
      </div>
    </div>

    <div
      v-for="[category, items] in categoryList"
      :key="category"
      class="home-category"
    >
      <h2 class="home-category-title">{{ category }}</h2>
      <div class="home-category-grid">
        <div
          v-for="art in items"
          :key="art.path"
          class="home-card"
          @click="goToArt(art.path)"
        >
          <div class="home-card-header">
            <span class="home-card-title">{{ art.metadata.title }}</span>
            <StatusBadge :status="art.metadata.status" />
          </div>
          <p v-if="art.metadata.description" class="home-card-desc">
            {{ art.metadata.description }}
          </p>
          <div class="home-card-footer">
            <span class="home-card-meta">
              {{ art.variants.length }} variant{{ art.variants.length !== 1 ? 's' : '' }}
            </span>
            <div class="home-card-tags">
              <span
                v-for="tag in art.metadata.tags.slice(0, 3)"
                :key="tag"
                class="home-card-tag"
              >
                #{{ tag }}
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div v-if="arts.length === 0" class="home-empty">
      <div class="home-empty-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M4 5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V5Z" />
          <path d="M4 13a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-6Z" />
          <path d="M16 13a1 1 0 0 1 1-1h2a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1v-6Z" />
        </svg>
      </div>
      <h2>No components found</h2>
      <p>Create <code>*.art.vue</code> files to get started</p>
    </div>
  </div>
</template>

<style scoped>
.home {
  max-width: 1400px;
  margin: 0 auto;
  padding: 2rem;
}

.home-header {
  margin-bottom: 3rem;
}

.home-title {
  font-size: 2rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
}

.home-description {
  color: var(--musea-text-muted);
  font-size: 1rem;
  margin-bottom: 1.5rem;
}

.home-stats {
  display: flex;
  gap: 2rem;
}

.home-stat {
  display: flex;
  flex-direction: column;
}

.home-stat-value {
  font-size: 1.75rem;
  font-weight: 700;
  color: var(--musea-accent);
  font-variant-numeric: tabular-nums;
}

.home-stat-label {
  font-size: 0.75rem;
  color: var(--musea-text-muted);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.home-category {
  margin-bottom: 2.5rem;
}

.home-category-title {
  font-size: 1.125rem;
  font-weight: 600;
  margin-bottom: 1rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--musea-border);
}

.home-category-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 1rem;
}

.home-card {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-lg);
  padding: 1.25rem;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.home-card:hover {
  border-color: var(--musea-text-muted);
  box-shadow: var(--musea-shadow);
  transform: translateY(-2px);
}

.home-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.5rem;
}

.home-card-title {
  font-weight: 600;
  font-size: 0.9375rem;
}

.home-card-desc {
  color: var(--musea-text-muted);
  font-size: 0.8125rem;
  margin-bottom: 0.75rem;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.home-card-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.home-card-meta {
  font-size: 0.75rem;
  color: var(--musea-text-muted);
}

.home-card-tags {
  display: flex;
  gap: 0.375rem;
}

.home-card-tag {
  font-size: 0.6875rem;
  color: var(--musea-text-muted);
  background: var(--musea-bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
}

.home-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 400px;
  text-align: center;
  color: var(--musea-text-muted);
}

.home-empty-icon {
  width: 80px;
  height: 80px;
  background: var(--musea-bg-secondary);
  border-radius: var(--musea-radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1.5rem;
}

.home-empty-icon svg {
  width: 40px;
  height: 40px;
}

.home-empty h2 {
  font-size: 1.125rem;
  margin-bottom: 0.5rem;
  color: var(--musea-text);
}

.home-empty code {
  background: var(--musea-bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-size: 0.875rem;
}
</style>
