<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useArts } from '../composables/useArts'
import { useActions } from '../composables/useActions'
import VariantCard from '../components/VariantCard.vue'
import StatusBadge from '../components/StatusBadge.vue'
import PropsPanel from '../components/PropsPanel.vue'
import DocumentationPanel from '../components/DocumentationPanel.vue'
import A11yBadge from '../components/A11yBadge.vue'
import AddonToolbar from '../components/AddonToolbar.vue'
import ActionsPanel from '../components/ActionsPanel.vue'

const route = useRoute()
const { getArt, load } = useArts()
const { init: initActions } = useActions()

const activeTab = ref<'variants' | 'props' | 'docs' | 'a11y' | 'actions'>('variants')

const artPath = computed(() => route.params.path as string)
const art = computed(() => getArt(artPath.value))

onMounted(() => {
  load()
  initActions()
})

watch(artPath, () => {
  activeTab.value = 'variants'
})
</script>

<template>
  <div v-if="art" class="component-view">
    <div class="component-header">
      <div class="component-title-row">
        <h1 class="component-title">{{ art.metadata.title }}</h1>
        <StatusBadge :status="art.metadata.status" />
      </div>
      <p v-if="art.metadata.description" class="component-description">
        {{ art.metadata.description }}
      </p>
      <div class="component-meta">
        <span class="meta-tag">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="3" y="3" width="7" height="7" />
            <rect x="14" y="3" width="7" height="7" />
            <rect x="3" y="14" width="7" height="7" />
            <rect x="14" y="14" width="7" height="7" />
          </svg>
          {{ art.variants.length }} variant{{ art.variants.length !== 1 ? 's' : '' }}
        </span>
        <span v-if="art.metadata.category" class="meta-tag">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
          </svg>
          {{ art.metadata.category }}
        </span>
        <span
          v-for="tag in art.metadata.tags"
          :key="tag"
          class="meta-tag"
        >
          #{{ tag }}
        </span>
      </div>
    </div>

    <AddonToolbar />

    <div class="component-tabs">
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'variants' }"
        @click="activeTab = 'variants'"
      >
        Variants
      </button>
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'props' }"
        @click="activeTab = 'props'"
      >
        Props
      </button>
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'docs' }"
        @click="activeTab = 'docs'"
      >
        Docs
      </button>
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'a11y' }"
        @click="activeTab = 'a11y'"
      >
        A11y
        <A11yBadge :art-path="art.path" />
      </button>
      <button
        class="tab-btn"
        :class="{ active: activeTab === 'actions' }"
        @click="activeTab = 'actions'"
      >
        Actions
      </button>
    </div>

    <div class="component-content">
      <div v-if="activeTab === 'variants'" class="gallery-grid">
        <VariantCard
          v-for="variant in art.variants"
          :key="variant.name"
          :art-path="art.path"
          :variant="variant"
        />
      </div>

      <PropsPanel
        v-if="activeTab === 'props'"
        :art-path="art.path"
      />

      <DocumentationPanel
        v-if="activeTab === 'docs'"
        :art-path="art.path"
      />

      <div v-if="activeTab === 'a11y'" class="a11y-placeholder">
        <p class="a11y-info">
          Run <code>musea-vrt --a11y</code> to generate accessibility reports, or view results in the A11y tab after running VRT tests.
        </p>
      </div>

      <ActionsPanel v-if="activeTab === 'actions'" />
    </div>
  </div>

  <div v-else class="component-not-found">
    <h2>Component not found</h2>
    <p>The requested component could not be found.</p>
    <router-link to="/" class="back-link">Back to home</router-link>
  </div>
</template>

<style scoped>
.component-view {
  max-width: 1400px;
  margin: 0 auto;
  padding: 2rem;
}

.component-header {
  margin-bottom: 1.5rem;
}

.component-title-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
}

.component-title {
  font-size: 1.5rem;
  font-weight: 700;
}

.component-description {
  color: var(--musea-text-muted);
  font-size: 0.9375rem;
  max-width: 600px;
  margin-bottom: 0.75rem;
}

.component-meta {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.meta-tag {
  display: inline-flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.625rem;
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  font-size: 0.75rem;
  color: var(--musea-text-muted);
}

.meta-tag svg {
  width: 12px;
  height: 12px;
}

.component-view :deep(.addon-toolbar) {
  margin-bottom: 1rem;
}

.component-tabs {
  display: flex;
  gap: 0.25rem;
  border-bottom: 1px solid var(--musea-border);
  margin-bottom: 1.5rem;
}

.tab-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  background: none;
  border: none;
  color: var(--musea-text-muted);
  font-size: 0.875rem;
  font-weight: 500;
  padding: 0.75rem 1rem;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all var(--musea-transition);
}

.tab-btn:hover {
  color: var(--musea-text);
}

.tab-btn.active {
  color: var(--musea-accent);
  border-bottom-color: var(--musea-accent);
}

.gallery-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 1.25rem;
}

.a11y-placeholder {
  padding: 2rem;
  text-align: center;
}

.a11y-info {
  color: var(--musea-text-muted);
  font-size: 0.875rem;
}

.a11y-info code {
  background: var(--musea-bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-family: var(--musea-font-mono);
}

.component-not-found {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 400px;
  text-align: center;
  color: var(--musea-text-muted);
}

.component-not-found h2 {
  color: var(--musea-text);
  margin-bottom: 0.5rem;
}

.back-link {
  margin-top: 1rem;
  color: var(--musea-accent);
  text-decoration: underline;
}
</style>
