<script setup lang="ts">
import { computed, ref, watch, onMounted } from 'vue'
import type { ArtVariant } from '../../src/types.js'
import { getPreviewUrl } from '../api'
import { useAddons } from '../composables/useAddons'
import { sendMessage } from '../composables/usePostMessage'

const props = defineProps<{
  artPath: string
  variant: ArtVariant
  componentName?: string
}>()

const emit = defineEmits<{
  (e: 'event', event: { type: string; target?: string; payload?: unknown }): void
}>()

const iframeRef = ref<HTMLIFrameElement | null>(null)
const iframeReady = ref(false)

const previewUrl = computed(() => getPreviewUrl(props.artPath, props.variant.name))

const {
  outlineEnabled,
  measureEnabled,
  getEffectiveBackground,
  getEffectiveViewport,
  openFullscreen,
} = useAddons()

const viewportStyle = computed(() => {
  const vp = getEffectiveViewport()
  if (vp.width === '100%') {
    return { width: '100%', height: '100%' }
  }
  return { width: vp.width, height: vp.height }
})

const isCustomViewport = computed(() => {
  const vp = getEffectiveViewport()
  return vp.width !== '100%'
})

function onIframeLoad() {
  iframeReady.value = true
  syncAllState()
  setupEventCapture()
}

function syncAllState() {
  const iframe = iframeRef.value
  if (!iframe) return

  const bg = getEffectiveBackground()
  if (bg.color) {
    sendMessage(iframe, 'musea:set-background', { color: bg.color, pattern: bg.pattern })
  }

  sendMessage(iframe, 'musea:toggle-outline', { enabled: outlineEnabled.value })
  sendMessage(iframe, 'musea:toggle-measure', { enabled: measureEnabled.value })
}

function setupEventCapture() {
  const iframe = iframeRef.value
  if (!iframe) return

  // Request event capture from iframe
  sendMessage(iframe, 'musea:enable-event-capture', { enabled: true })
}

// Handle messages from iframe
function handleIframeMessage(event: MessageEvent) {
  if (event.source !== iframeRef.value?.contentWindow) return

  if (event.data?.type === 'musea:dom-event') {
    emit('event', {
      type: event.data.eventType,
      target: event.data.target,
      payload: event.data.payload
    })
  }
}

onMounted(() => {
  window.addEventListener('message', handleIframeMessage)
})

watch(() => getEffectiveBackground(), (bg) => {
  const iframe = iframeRef.value
  if (!iframe || !iframeReady.value) return
  sendMessage(iframe, 'musea:set-background', { color: bg.color, pattern: bg.pattern })
}, { deep: true })

watch(outlineEnabled, (enabled) => {
  const iframe = iframeRef.value
  if (!iframe || !iframeReady.value) return
  sendMessage(iframe, 'musea:toggle-outline', { enabled })
})

watch(measureEnabled, (enabled) => {
  const iframe = iframeRef.value
  if (!iframe || !iframeReady.value) return
  sendMessage(iframe, 'musea:toggle-measure', { enabled })
})

// Re-setup when variant changes
watch(() => props.variant.name, () => {
  iframeReady.value = false
})

function resolveSelfReferences(template: string): string {
  if (!props.componentName) return template
  return template
    .replace(/<Self(\s|>|\/)/g, `<${props.componentName}$1`)
    .replace(/<\/Self>/g, `</${props.componentName}>`)
}

const resolvedTemplate = computed(() => resolveSelfReferences(props.variant.template))

const copied = ref(false)

async function copyTemplate() {
  try {
    await navigator.clipboard.writeText(resolvedTemplate.value)
    copied.value = true
    setTimeout(() => { copied.value = false }, 2000)
  } catch {
    // fallback
  }
}

function openInNewTab() {
  window.open(previewUrl.value, '_blank')
}
</script>

<template>
  <div class="variant-preview-container">
    <div class="preview-area" :class="{ 'viewport-mode': isCustomViewport }">
      <iframe
        ref="iframeRef"
        :src="previewUrl"
        :title="variant.name"
        :style="viewportStyle"
        @load="onIframeLoad"
      />
    </div>

    <div class="preview-toolbar">
      <div class="toolbar-left">
        <span class="variant-name">{{ variant.name }}</span>
        <span v-if="variant.isDefault" class="variant-badge">Default</span>
      </div>
      <div class="toolbar-actions">
        <button
          type="button"
          class="toolbar-btn"
          :class="{ active: copied }"
          :title="copied ? 'Copied!' : 'Copy template'"
          @click="copyTemplate"
        >
          <svg v-if="!copied" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
          </svg>
          <svg v-else viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="20 6 9 17 4 12" />
          </svg>
        </button>
        <button
          type="button"
          class="toolbar-btn"
          title="Fullscreen"
          @click="openFullscreen(artPath, variant.name)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3" />
          </svg>
        </button>
        <button
          type="button"
          class="toolbar-btn"
          title="Open in new tab"
          @click="openInNewTab"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
            <polyline points="15 3 21 3 21 9" />
            <line x1="10" y1="14" x2="21" y2="3" />
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.variant-preview-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--musea-bg-tertiary);
}

.preview-area {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 300px;
  overflow: hidden;
}

.preview-area.viewport-mode {
  overflow: auto;
}

.preview-area iframe {
  width: 100%;
  height: 100%;
  border: none;
  background: white;
}

.preview-area.viewport-mode iframe {
  flex-shrink: 0;
}

.preview-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.625rem 1rem;
  background: var(--musea-bg-secondary);
  border-top: 1px solid var(--musea-border);
  flex-shrink: 0;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.variant-name {
  font-weight: 600;
  font-size: 0.875rem;
  color: var(--musea-text);
}

.variant-badge {
  font-size: 0.5625rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 0.125rem 0.375rem;
  border-radius: 3px;
  background: var(--musea-accent-subtle);
  color: var(--musea-accent);
}

.toolbar-actions {
  display: flex;
  gap: 0.375rem;
}

.toolbar-btn {
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
  transition: all 0.15s;
}

.toolbar-btn:hover {
  background: var(--musea-bg-primary);
  color: var(--musea-text);
}

.toolbar-btn.active {
  color: var(--musea-accent);
  background: var(--musea-accent-subtle);
}

.toolbar-btn svg {
  width: 14px;
  height: 14px;
}
</style>
