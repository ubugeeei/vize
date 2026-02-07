<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useArts } from '../composables/useArts'
import { useActions } from '../composables/useActions'
import { useAddons } from '../composables/useAddons'
import VariantCard from '../components/VariantCard.vue'
import StatusBadge from '../components/StatusBadge.vue'
import PropsPanel from '../components/PropsPanel.vue'
import DocumentationPanel from '../components/DocumentationPanel.vue'
import A11yBadge from '../components/A11yBadge.vue'
import AddonToolbar from '../components/AddonToolbar.vue'
import ActionsPanel from '../components/ActionsPanel.vue'
import FullscreenPreview from '../components/FullscreenPreview.vue'

const route = useRoute()
const { getArt, load } = useArts()
const { events, init: initActions, clear: clearActions } = useActions()
const { gridDensity } = useAddons()

const activeTab = ref<'variants' | 'props' | 'docs' | 'a11y'>('variants')
const actionCount = computed(() => events.value.length)
const actionsExpanded = ref(false)

const gridClass = computed(() => `gallery-grid density-${gridDensity.value}`)

const artPath = computed(() => route.params.path as string)
const art = computed(() => getArt(artPath.value))

onMounted(() => {
  load()
  initActions()
})

watch(artPath, () => {
  activeTab.value = 'variants'
  clearActions()
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
    </div>

    <div class="component-content">
      <div v-if="activeTab === 'variants'" :class="gridClass">
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
        :default-variant-name="art.variants.find(v => v.isDefault)?.name || art.variants[0]?.name"
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

    </div>

    <!-- Actions Footer Panel -->
    <div class="actions-footer" :class="{ expanded: actionsExpanded }">
      <button class="actions-footer-toggle" @click="actionsExpanded = !actionsExpanded">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="14" height="14">
          <polyline :points="actionsExpanded ? '18 15 12 9 6 15' : '6 9 12 15 18 9'" />
        </svg>
        Actions
        <span v-if="actionCount > 0" class="action-count-badge">{{ actionCount > 99 ? '99+' : actionCount }}</span>
      </button>
      <div v-if="actionsExpanded" class="actions-footer-content">
        <ActionsPanel />
      </div>
    </div>

    <FullscreenPreview />
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

.action-count-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 0.375rem;
  border-radius: 9px;
  background: var(--musea-accent);
  color: #fff;
  font-size: 0.625rem;
  font-weight: 700;
  line-height: 1;
}

.gallery-grid {
  display: grid;
  gap: 1.25rem;
}

.gallery-grid.density-compact {
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 0.75rem;
}

.gallery-grid.density-comfortable {
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 1.25rem;
}

.gallery-grid.density-spacious {
  grid-template-columns: repeat(auto-fill, minmax(480px, 1fr));
  gap: 1.75rem;
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

.actions-footer {
  margin-top: 1.5rem;
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  overflow: hidden;
}

.actions-footer-toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.625rem 1rem;
  background: var(--musea-bg-secondary);
  border: none;
  color: var(--musea-text-muted);
  font-size: 0.8125rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--musea-transition);
}

.actions-footer-toggle:hover {
  background: var(--musea-bg-tertiary);
  color: var(--musea-text);
}

.actions-footer-content {
  border-top: 1px solid var(--musea-border);
  max-height: 300px;
  overflow-y: auto;
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
