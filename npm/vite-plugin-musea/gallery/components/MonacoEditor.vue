<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import type * as Monaco from 'monaco-editor'

const props = defineProps<{
  modelValue: string
  language?: string
  theme?: string
  height?: string
  readOnly?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const containerRef = ref<HTMLDivElement | null>(null)
let editor: Monaco.editor.IStandaloneCodeEditor | null = null
let monaco: typeof Monaco | null = null

onMounted(async () => {
  if (!containerRef.value) return

  // Dynamic import monaco-editor
  monaco = await import('monaco-editor')

  // Configure monaco environment for workers
  self.MonacoEnvironment = {
    getWorker(_workerId: string, label: string) {
      const getWorkerModule = (moduleUrl: string, label: string) => {
        return new Worker(
          new URL(moduleUrl, import.meta.url),
          { type: 'module', name: label }
        )
      }
      switch (label) {
        case 'json':
          return getWorkerModule('monaco-editor/esm/vs/language/json/json.worker?worker', label)
        case 'css':
        case 'scss':
        case 'less':
          return getWorkerModule('monaco-editor/esm/vs/language/css/css.worker?worker', label)
        case 'html':
        case 'handlebars':
        case 'razor':
          return getWorkerModule('monaco-editor/esm/vs/language/html/html.worker?worker', label)
        case 'typescript':
        case 'javascript':
          return getWorkerModule('monaco-editor/esm/vs/language/typescript/ts.worker?worker', label)
        default:
          return getWorkerModule('monaco-editor/esm/vs/editor/editor.worker?worker', label)
      }
    }
  }

  // Define custom dark theme matching musea
  monaco.editor.defineTheme('musea-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': '#1a1a1a',
      'editor.foreground': '#e5e5e5',
      'editor.lineHighlightBackground': '#252525',
      'editorCursor.foreground': '#e07048',
      'editor.selectionBackground': '#3d3d3d',
    }
  })

  editor = monaco.editor.create(containerRef.value, {
    value: props.modelValue,
    language: props.language || 'html',
    theme: props.theme || 'musea-dark',
    minimap: { enabled: false },
    fontSize: 12,
    lineNumbers: 'on',
    lineNumbersMinChars: 3,
    scrollBeyondLastLine: false,
    wordWrap: 'on',
    automaticLayout: true,
    readOnly: props.readOnly || false,
    padding: { top: 8, bottom: 8 },
    renderLineHighlight: 'line',
    scrollbar: {
      vertical: 'auto',
      horizontal: 'auto',
      verticalScrollbarSize: 8,
      horizontalScrollbarSize: 8,
    },
    overviewRulerLanes: 0,
    hideCursorInOverviewRuler: true,
    overviewRulerBorder: false,
    folding: false,
    tabSize: 2,
  })

  editor.onDidChangeModelContent(() => {
    const value = editor?.getValue() || ''
    emit('update:modelValue', value)
  })
})

onBeforeUnmount(() => {
  editor?.dispose()
})

watch(() => props.modelValue, (newValue) => {
  if (editor && editor.getValue() !== newValue) {
    editor.setValue(newValue)
  }
})

watch(() => props.language, (newLang) => {
  if (editor && monaco && newLang) {
    const model = editor.getModel()
    if (model) {
      monaco.editor.setModelLanguage(model, newLang)
    }
  }
})
</script>

<template>
  <div
    ref="containerRef"
    class="monaco-container"
    :style="{ height: height || '120px' }"
  />
</template>

<style scoped>
.monaco-container {
  border: 1px solid var(--musea-border);
  border-radius: var(--musea-radius-sm);
  overflow: hidden;
}
</style>
