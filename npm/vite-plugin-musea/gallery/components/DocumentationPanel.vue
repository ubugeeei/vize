<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { Marked } from 'marked'
import { markedHighlight } from 'marked-highlight'
import hljs from 'highlight.js/lib/core'
import xml from 'highlight.js/lib/languages/xml'
import javascript from 'highlight.js/lib/languages/javascript'
import typescript from 'highlight.js/lib/languages/typescript'
import css from 'highlight.js/lib/languages/css'
import bash from 'highlight.js/lib/languages/bash'
import { fetchDocs } from '../api'

hljs.registerLanguage('xml', xml)
hljs.registerLanguage('html', xml)
hljs.registerLanguage('vue', xml)
hljs.registerLanguage('javascript', javascript)
hljs.registerLanguage('js', javascript)
hljs.registerLanguage('typescript', typescript)
hljs.registerLanguage('ts', typescript)
hljs.registerLanguage('css', css)
hljs.registerLanguage('bash', bash)
hljs.registerLanguage('sh', bash)

const markedInstance = new Marked(
  markedHighlight({
    highlight(code: string, lang: string) {
      if (lang && hljs.getLanguage(lang)) {
        return hljs.highlight(code, { language: lang }).value
      }
      return code
    },
  }),
)

const props = defineProps<{
  artPath: string
}>()

const markdown = ref('')
const loading = ref(false)
const error = ref<string | null>(null)

const renderedHtml = computed(() => {
  if (!markdown.value) return ''
  return markedInstance.parse(markdown.value) as string
})

watch(() => props.artPath, async (path) => {
  if (!path) return
  loading.value = true
  error.value = null
  try {
    const data = await fetchDocs(path)
    markdown.value = data.markdown
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e)
  } finally {
    loading.value = false
  }
}, { immediate: true })
</script>

<template>
  <div class="docs-panel">
    <div v-if="loading" class="docs-loading">
      <div class="loading-spinner" />
      Loading documentation...
    </div>

    <div v-else-if="error" class="docs-error">
      {{ error }}
    </div>

    <div v-else-if="markdown" class="docs-content">
      <div class="docs-markdown" v-html="renderedHtml" />
    </div>

    <div v-else class="docs-empty">
      <p>No documentation available for this component.</p>
    </div>
  </div>
</template>

<style scoped>
.docs-panel {
  padding: 0.5rem;
}

.docs-loading {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  justify-content: center;
  min-height: 200px;
  color: var(--musea-text-muted);
}

.loading-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--musea-border);
  border-top-color: var(--musea-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.docs-error {
  padding: 1rem;
  color: var(--musea-error);
  background: rgba(248, 113, 113, 0.1);
  border: 1px solid rgba(248, 113, 113, 0.2);
  border-radius: var(--musea-radius-md);
  font-size: 0.8125rem;
}

.docs-content {
  background: var(--musea-bg-secondary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  overflow: hidden;
}

.docs-markdown {
  padding: 1.5rem;
  font-size: 0.875rem;
  line-height: 1.7;
  color: var(--musea-text-secondary);
}

.docs-markdown :deep(h1) {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--musea-text);
  margin-bottom: 1rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--musea-border);
}

.docs-markdown :deep(h2) {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--musea-text);
  margin-top: 1.5rem;
  margin-bottom: 0.75rem;
}

.docs-markdown :deep(h3) {
  font-size: 1rem;
  font-weight: 600;
  color: var(--musea-text);
  margin-top: 1.25rem;
  margin-bottom: 0.5rem;
}

.docs-markdown :deep(p) {
  margin-bottom: 0.75rem;
}

.docs-markdown :deep(ul),
.docs-markdown :deep(ol) {
  padding-left: 1.5rem;
  margin-bottom: 0.75rem;
}

.docs-markdown :deep(li) {
  margin-bottom: 0.25rem;
}

.docs-markdown :deep(code) {
  background: var(--musea-bg-tertiary);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
  font-size: 0.8125rem;
}

.docs-markdown :deep(pre) {
  background: var(--musea-bg-primary);
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-md);
  padding: 1rem;
  margin-bottom: 1rem;
  overflow-x: auto;
}

.docs-markdown :deep(pre code) {
  background: none;
  padding: 0;
  font-size: 0.8125rem;
  line-height: 1.6;
}

.docs-markdown :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: 1rem;
}

.docs-markdown :deep(th),
.docs-markdown :deep(td) {
  padding: 0.5rem 0.75rem;
  border: 1px solid var(--musea-border);
  text-align: left;
  font-size: 0.8125rem;
}

.docs-markdown :deep(th) {
  background: var(--musea-bg-tertiary);
  font-weight: 600;
  color: var(--musea-text);
}

.docs-markdown :deep(blockquote) {
  border-left: 3px solid var(--musea-accent);
  padding-left: 1rem;
  margin: 0.75rem 0;
  color: var(--musea-text-muted);
}

.docs-markdown :deep(hr) {
  border: none;
  border-top: 1px solid var(--musea-border);
  margin: 1.5rem 0;
}

.docs-markdown :deep(a) {
  color: var(--musea-accent);
  text-decoration: underline;
}

.docs-markdown :deep(strong) {
  color: var(--musea-text);
  font-weight: 600;
}

.docs-empty {
  padding: 2rem;
  text-align: center;
  color: var(--musea-text-muted);
  font-size: 0.875rem;
}
</style>
