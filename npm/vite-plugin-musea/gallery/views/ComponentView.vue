<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import { useRoute } from "vue-router";
import { mdiViewGrid, mdiFolder, mdiChevronUp, mdiChevronDown } from "@mdi/js";
import { useArts } from "../composables/useArts";
import { useActions } from "../composables/useActions";
import { useAddons } from "../composables/useAddons";
import { useEventCapture } from "../composables/useEventCapture";
import MdiIcon from "../components/MdiIcon.vue";
import VariantCard from "../components/VariantCard.vue";
import VariantTabs from "../components/VariantTabs.vue";
import StatusBadge from "../components/StatusBadge.vue";
import PropsPanel from "../components/PropsPanel.vue";
import DocumentationPanel from "../components/DocumentationPanel.vue";
import A11yBadge from "../components/A11yBadge.vue";
import A11yPanel from "../components/A11yPanel.vue";
import VrtPanel from "../components/VrtPanel.vue";
import AddonToolbar from "../components/AddonToolbar.vue";
import ActionsPanel from "../components/ActionsPanel.vue";
import FullscreenPreview from "../components/FullscreenPreview.vue";

const route = useRoute();
const { getArt, load } = useArts();
const {
  events,
  init: initActions,
  clear: clearActions,
  setCurrentVariant: setActionsVariant,
} = useActions();
const { gridDensity } = useAddons();
const { setCurrentVariant } = useEventCapture();

const activeTab = ref<"variants" | "props" | "docs" | "a11y" | "vrt">(
  "variants",
);
const actionCount = computed(() => events.value.length);
const actionsExpanded = ref(false);

// Currently selected variant name
const selectedVariantName = ref<string>("");

const gridClass = computed(() => `gallery-grid density-${gridDensity.value}`);

const artPath = computed(() => route.params.path as string);
const art = computed(() => getArt(artPath.value));

// Get the currently selected variant
const selectedVariant = computed(() => {
  if (!art.value) return null;
  return (
    art.value.variants.find((v) => v.name === selectedVariantName.value) ||
    art.value.variants[0]
  );
});

// Initialize selected variant when art changes
watch(
  art,
  (newArt) => {
    if (newArt) {
      const defaultVariant =
        newArt.variants.find((v) => v.isDefault) || newArt.variants[0];
      selectedVariantName.value = defaultVariant?.name || "";
      setCurrentVariant(selectedVariantName.value);
      setActionsVariant(selectedVariantName.value);
    }
  },
  { immediate: true },
);

// Update event capture when variant changes
watch(selectedVariantName, (name) => {
  setCurrentVariant(name);
  setActionsVariant(name);
});

onMounted(() => {
  load();
  initActions();
});

watch(artPath, () => {
  activeTab.value = "variants";
  clearActions();
});

const handleVariantSelect = (variantName: string) => {
  selectedVariantName.value = variantName;
};
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
          <MdiIcon :path="mdiViewGrid" :size="12" />
          {{ art.variants.length }} variant{{
            art.variants.length !== 1 ? "s" : ""
          }}
        </span>
        <span v-if="art.metadata.category" class="meta-tag">
          <MdiIcon :path="mdiFolder" :size="12" />
          {{ art.metadata.category }}
        </span>
        <span v-for="tag in art.metadata.tags" :key="tag" class="meta-tag">
          #{{ tag }}
        </span>
      </div>
    </div>

    <AddonToolbar />

    <div class="component-tabs">
      <button
        type="button"
        class="tab-btn"
        :class="{ active: activeTab === 'variants' }"
        @click="activeTab = 'variants'"
      >
        Variants
      </button>
      <button
        type="button"
        class="tab-btn"
        :class="{ active: activeTab === 'props' }"
        @click="activeTab = 'props'"
      >
        Props
      </button>
      <button
        type="button"
        class="tab-btn"
        :class="{ active: activeTab === 'docs' }"
        @click="activeTab = 'docs'"
      >
        Docs
      </button>
      <button
        type="button"
        class="tab-btn"
        :class="{ active: activeTab === 'a11y' }"
        @click="activeTab = 'a11y'"
      >
        A11y
        <A11yBadge :art-path="art.path" />
      </button>
      <button
        type="button"
        class="tab-btn"
        :class="{ active: activeTab === 'vrt' }"
        @click="activeTab = 'vrt'"
      >
        VRT
      </button>
    </div>

    <div class="component-content">
      <!-- Variants Tab: Show variant tabs + single preview -->
      <div v-if="activeTab === 'variants'" class="variants-view">
        <VariantTabs
          :variants="art.variants"
          :selected-variant="selectedVariantName"
          @select="handleVariantSelect"
        />
        <div class="variant-preview-area">
          <VariantCard
            v-if="selectedVariant"
            :key="selectedVariant.name"
            :art-path="art.path"
            :variant="selectedVariant"
            :component-name="art.metadata.title"
          />
        </div>
      </div>

      <PropsPanel
        v-if="activeTab === 'props'"
        :art-path="art.path"
        :default-variant-name="
          art.variants.find((v) => v.isDefault)?.name || art.variants[0]?.name
        "
      />

      <DocumentationPanel v-if="activeTab === 'docs'" :art-path="art.path" />

      <A11yPanel
        v-if="activeTab === 'a11y'"
        :art-path="art.path"
        :default-variant-name="selectedVariant?.name"
      />

      <VrtPanel
        v-if="activeTab === 'vrt'"
        :art-path="art.path"
        :default-variant-name="selectedVariant?.name"
      />
    </div>

    <!-- Actions Footer Panel (sticky bottom) -->
    <div class="actions-footer" :class="{ expanded: actionsExpanded }">
      <div v-if="actionsExpanded" class="actions-footer-content">
        <ActionsPanel />
      </div>
      <button
        type="button"
        class="actions-footer-toggle"
        @click="actionsExpanded = !actionsExpanded"
      >
        <MdiIcon
          :path="actionsExpanded ? mdiChevronDown : mdiChevronUp"
          :size="14"
        />
        Actions
        <span v-if="actionCount > 0" class="action-count-badge">{{
          actionCount > 99 ? "99+" : actionCount
        }}</span>
      </button>
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

.component-content {
  min-height: 0;
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

.variants-view {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.variant-preview-area {
  min-height: 0;
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

.actions-footer {
  position: sticky;
  bottom: 0;
  margin: 0 -2rem -2rem;
  background: var(--musea-bg-primary);
  border-top: 1px solid var(--musea-border);
  z-index: 10;
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
  border-bottom: 1px solid var(--musea-border);
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
