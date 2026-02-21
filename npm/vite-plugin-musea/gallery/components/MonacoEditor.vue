<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, computed } from 'vue'
import type * as Monaco from 'monaco-editor'
import { useTheme } from '../composables/useTheme'
import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import JsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
import CssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker'
import HtmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker'
import TsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

const props = defineProps<{
  modelValue: string
  language?: string
  theme?: string
  height?: string
  readOnly?: boolean
  completionItems?: string[]
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const { resolvedTheme } = useTheme()

const containerRef = ref<HTMLDivElement | null>(null)
let editor: Monaco.editor.IStandaloneCodeEditor | null = null
let monaco: typeof Monaco | null = null
let completionDisposable: Monaco.IDisposable | null = null

const monacoTheme = computed(() => {
  const resolved = resolvedTheme.value
  return resolved === 'dark' ? 'musea-dark' : 'musea-light'
})

onMounted(async () => {
  if (!containerRef.value) return

  // Configure monaco environment for workers using Vite ?worker imports
  self.MonacoEnvironment = {
    getWorker(_workerId: string, label: string) {
      switch (label) {
        case 'json':
          return new JsonWorker()
        case 'css':
        case 'scss':
        case 'less':
          return new CssWorker()
        case 'html':
        case 'handlebars':
        case 'razor':
          return new HtmlWorker()
        case 'typescript':
        case 'javascript':
          return new TsWorker()
        default:
          return new EditorWorker()
      }
    }
  }

  // Dynamic import monaco-editor
  monaco = await import('monaco-editor')

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

  // Define custom light theme matching musea brand
  monaco.editor.defineTheme('musea-light', {
    base: 'vs',
    inherit: true,
    rules: [],
    colors: {
      'editor.background': '#ddd9cd',
      'editor.foreground': '#121212',
      'editor.lineHighlightBackground': '#d4d0c4',
      'editorCursor.foreground': '#121212',
      'editor.selectionBackground': '#c8c4b8',
    }
  })

  // Register design token completion provider
  if (props.completionItems && props.completionItems.length > 0) {
    registerCompletions(monaco, props.completionItems)
  }

  editor = monaco.editor.create(containerRef.value, {
    value: props.modelValue,
    language: props.language || 'html',
    theme: props.theme || monacoTheme.value,
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

function registerCompletions(m: typeof Monaco, items: string[]) {
  completionDisposable?.dispose()
  completionDisposable = m.languages.registerCompletionItemProvider('css', {
    triggerCharacters: ['-', '('],
    provideCompletionItems(model, position) {
      const lineContent = model.getLineContent(position.lineNumber)
      const textUntilPos = lineContent.substring(0, position.column - 1)

      // Only suggest inside var() or after a colon (CSS value context)
      const inVar = /var\(\s*-*$/.test(textUntilPos)
      const afterColon = /:\s*[^;]*$/.test(textUntilPos)
      if (!inVar && !afterColon) return { suggestions: [] }

      const word = model.getWordUntilPosition(position)
      const range = {
        startLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endLineNumber: position.lineNumber,
        endColumn: word.endColumn,
      }

      const suggestions: Monaco.languages.CompletionItem[] = items.map(tokenPath => {
        const varName = `--${tokenPath.replace(/\./g, '-')}`
        return {
          label: varName,
          kind: m.languages.CompletionItemKind.Variable,
          detail: `Design token: ${tokenPath}`,
          insertText: inVar ? varName : `var(${varName})`,
          range,
          sortText: '0' + tokenPath,
        }
      })

      return { suggestions }
    }
  })

  // Also register for HTML (since SFC source is HTML language)
  completionDisposable = m.languages.registerCompletionItemProvider('html', {
    triggerCharacters: ['-', '('],
    provideCompletionItems(model, position) {
      const lineContent = model.getLineContent(position.lineNumber)
      const textUntilPos = lineContent.substring(0, position.column - 1)

      const inVar = /var\(\s*-*$/.test(textUntilPos)
      const afterColon = /:\s*[^;]*$/.test(textUntilPos)
      if (!inVar && !afterColon) return { suggestions: [] }

      const word = model.getWordUntilPosition(position)
      const range = {
        startLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endLineNumber: position.lineNumber,
        endColumn: word.endColumn,
      }

      const suggestions: Monaco.languages.CompletionItem[] = items.map(tokenPath => {
        const varName = `--${tokenPath.replace(/\./g, '-')}`
        return {
          label: varName,
          kind: m.languages.CompletionItemKind.Variable,
          detail: `Design token: ${tokenPath}`,
          insertText: inVar ? varName : `var(${varName})`,
          range,
          sortText: '0' + tokenPath,
        }
      })

      return { suggestions }
    }
  })
}

onBeforeUnmount(() => {
  completionDisposable?.dispose()
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

watch(() => props.completionItems, (newItems) => {
  if (monaco && newItems && newItems.length > 0) {
    registerCompletions(monaco, newItems)
  }
})

watch(monacoTheme, (newTheme) => {
  if (monaco && !props.theme) {
    monaco.editor.setTheme(newTheme)
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
