<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  mdiContentCopy,
  mdiCheck,
  mdiCodeTags,
  mdiFullscreen,
  mdiOpenInNew,
} from "@mdi/js";
import type { ArtVariant } from "../../src/types.js";
import { getPreviewUrl } from "../api";
import { useAddons } from "../composables/useAddons";
import { sendMessage } from "../composables/usePostMessage";
import VariantSourceCode from "./VariantSourceCode.vue";
import MdiIcon from "./MdiIcon.vue";

const props = defineProps<{
  artPath: string;
  variant: ArtVariant;
  componentName?: string;
}>();

const copied = ref(false);

function resolveSelfReferences(template: string): string {
  if (!props.componentName) return template;
  return template
    .replace(/<Self(\s|>|\/)/g, `<${props.componentName}$1`)
    .replace(/<\/Self>/g, `</${props.componentName}>`);
}

const resolvedTemplate = computed(() =>
  resolveSelfReferences(props.variant.template),
);

async function copyTemplate() {
  try {
    await navigator.clipboard.writeText(resolvedTemplate.value);
    copied.value = true;
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch {
    // fallback
  }
}

const previewUrl = computed(() =>
  getPreviewUrl(props.artPath, props.variant.name),
);

const iframeRef = ref<HTMLIFrameElement | null>(null);
const iframeReady = ref(false);
const showSource = ref(false);

const {
  outlineEnabled,
  measureEnabled,
  getEffectiveBackground,
  getEffectiveViewport,
  openFullscreen,
} = useAddons();

const viewportStyle = computed(() => {
  const vp = getEffectiveViewport();
  if (vp.width === "100%") {
    return { width: "100%", height: "100%" };
  }
  return { width: vp.width, height: vp.height };
});

const isCustomViewport = computed(() => {
  const vp = getEffectiveViewport();
  return vp.width !== "100%";
});

// Listen for iframe ready
function onIframeLoad() {
  iframeReady.value = true;
  syncAllState();
}

function syncAllState() {
  const iframe = iframeRef.value;
  if (!iframe) return;

  // Sync background
  const bg = getEffectiveBackground();
  if (bg.color) {
    sendMessage(iframe, "musea:set-background", {
      color: bg.color,
      pattern: bg.pattern,
    });
  }

  // Sync outline
  sendMessage(iframe, "musea:toggle-outline", {
    enabled: outlineEnabled.value,
  });

  // Sync measure
  sendMessage(iframe, "musea:toggle-measure", {
    enabled: measureEnabled.value,
  });
}

// Watch addons state and send messages to iframe
watch(
  () => getEffectiveBackground(),
  (bg) => {
    const iframe = iframeRef.value;
    if (!iframe || !iframeReady.value) return;
    sendMessage(iframe, "musea:set-background", {
      color: bg.color,
      pattern: bg.pattern,
    });
  },
  { deep: true },
);

watch(outlineEnabled, (enabled) => {
  const iframe = iframeRef.value;
  if (!iframe || !iframeReady.value) return;
  sendMessage(iframe, "musea:toggle-outline", { enabled });
});

watch(measureEnabled, (enabled) => {
  const iframe = iframeRef.value;
  if (!iframe || !iframeReady.value) return;
  sendMessage(iframe, "musea:toggle-measure", { enabled });
});
</script>

<template>
  <div class="variant-card">
    <div class="variant-preview" :class="{ 'viewport-mode': isCustomViewport }">
      <iframe
        ref="iframeRef"
        :src="previewUrl"
        loading="lazy"
        :title="variant.name"
        :style="viewportStyle"
        @load="onIframeLoad"
      />
    </div>

    <div class="variant-info">
      <div class="variant-left">
        <span class="variant-name">{{ variant.name }}</span>
        <span v-if="variant.isDefault" class="variant-badge">Default</span>
      </div>
      <div class="variant-actions">
        <button
          type="button"
          class="variant-action-btn"
          :title="copied ? 'Copied!' : 'Copy template'"
          :class="{ active: copied }"
          @click="copyTemplate"
        >
          <MdiIcon v-if="!copied" :path="mdiContentCopy" :size="14" />
          <MdiIcon v-else :path="mdiCheck" :size="14" />
        </button>
        <button
          type="button"
          class="variant-action-btn"
          title="View source"
          :class="{ active: showSource }"
          @click="showSource = !showSource"
        >
          <MdiIcon :path="mdiCodeTags" :size="14" />
        </button>
        <button
          type="button"
          class="variant-action-btn"
          title="Fullscreen"
          @click="openFullscreen(artPath, variant.name)"
        >
          <MdiIcon :path="mdiFullscreen" :size="14" />
        </button>
        <button
          type="button"
          class="variant-action-btn"
          title="Open in new tab"
          @click="window.open(previewUrl, '_blank')"
        >
          <MdiIcon :path="mdiOpenInNew" :size="14" />
        </button>
      </div>
    </div>

    <VariantSourceCode v-if="showSource" :code="resolvedTemplate" />
  </div>
</template>

<script lang="ts">
// Expose window for template
const window = globalThis.window;
</script>

<style scoped>
.variant-card {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-lg);
  overflow: hidden;
  transition: all var(--musea-transition);
}

.variant-card:hover {
  border-color: var(--musea-text-muted);
  box-shadow: var(--musea-shadow);
  transform: translateY(-2px);
}

.variant-preview {
  aspect-ratio: 16 / 7;
  background: var(--musea-bg-tertiary);
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  overflow: hidden;
}

.variant-preview.viewport-mode {
  aspect-ratio: unset;
  min-height: 200px;
  max-height: 350px;
  overflow: auto;
}

.variant-preview iframe {
  width: 70%;
  height: 100%;
  border: none;
  background: white;
}

.variant-preview.viewport-mode iframe {
  flex-shrink: 0;
}

.variant-info {
  padding: 1rem;
  border-top: 1px solid var(--musea-border);
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.variant-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.variant-name {
  font-weight: 600;
  font-size: 0.875rem;
}

.variant-badge {
  font-size: 0.625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 0.1875rem 0.5rem;
  border-radius: 4px;
  background: var(--musea-accent-subtle);
  color: var(--musea-accent);
}

.variant-actions {
  display: flex;
  gap: 0.5rem;
}

.variant-action-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: var(--musea-bg-tertiary);
  border-radius: var(--musea-radius-sm);
  color: var(--musea-text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all var(--musea-transition);
}

.variant-action-btn:hover {
  background: var(--musea-bg-elevated);
  color: var(--musea-text);
}

.variant-action-btn.active {
  color: var(--musea-accent);
  background: var(--musea-accent-subtle);
}

.variant-action-btn svg {
  width: 14px;
  height: 14px;
}
</style>
