<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, shallowRef } from 'vue';
import * as monaco from 'monaco-editor';

export interface Diagnostic {
  message: string;
  startLine: number;
  startColumn: number;
  endLine?: number;
  endColumn?: number;
  severity: 'error' | 'warning' | 'info';
}

export interface ScopeDecoration {
  start: number;  // Character offset
  end: number;    // Character offset
  kind: string;   // Scope kind for styling
  kindStr?: string; // Human-readable description
}

const props = defineProps<{
  modelValue: string;
  language: string;
  diagnostics?: Diagnostic[];
  scopes?: ScopeDecoration[];
  readOnly?: boolean;
}>();

const emit = defineEmits<{
  'update:modelValue': [string]
}>();

const containerRef = ref<HTMLDivElement | null>(null);
const editorInstance = shallowRef<monaco.editor.IStandaloneCodeEditor | null>(null);
let isConfigured = false;

function configureMonaco() {
  if (isConfigured) return;
  isConfigured = true;

  // Register Vue language
  monaco.languages.register({ id: 'vue', extensions: ['.vue'] });

  // Set monarch tokenizer for Vue (HTML-based with Vue extensions)
  monaco.languages.setMonarchTokensProvider('vue', {
    defaultToken: '',
    tokenPostfix: '.vue',
    keywords: ['v-if', 'v-else', 'v-else-if', 'v-for', 'v-show', 'v-model', 'v-bind', 'v-on', 'v-slot', 'v-pre', 'v-once', 'v-memo', 'v-cloak'],
    tokenizer: {
      root: [
        [/<!--/, { token: 'comment', next: '@htmlComment' }],
        [/<script\s+setup\s+vapor[^>]*>/, { token: 'tag', next: '@script' }],
        [/<script\s+setup[^>]*>/, { token: 'tag', next: '@script' }],
        [/<script[^>]*>/, { token: 'tag', next: '@script' }],
        [/<style[^>]*>/, { token: 'tag', next: '@style' }],
        [/<template[^>]*>/, { token: 'tag', next: '@template' }],
        [/<\/?[\w-]+/, { token: 'tag', next: '@tag' }],
        [/\{\{/, { token: 'delimiter.bracket', next: '@interpolation' }],
      ],
      tag: [
        [/\s+/, ''],
        [/(v-[\w-]+|@[\w.-]+|:[\w.-]+|#[\w.-]+)/, 'attribute.name.vue'],
        [/[\w-]+/, 'attribute.name'],
        [/=/, 'delimiter'],
        [/"[^"]*"/, 'attribute.value'],
        [/'[^']*'/, 'attribute.value'],
        [/>/, { token: 'tag', next: '@pop' }],
        [/\/>/, { token: 'tag', next: '@pop' }],
      ],
      template: [
        [/<\/template>/, { token: 'tag', next: '@pop' }],
        [/<!--/, { token: 'comment', next: '@htmlComment' }],
        [/\{\{/, { token: 'delimiter.bracket', next: '@interpolation' }],
        [/<\/?[\w-]+/, { token: 'tag', next: '@tag' }],
        [/./, ''],
      ],
      htmlComment: [
        [/-->/, { token: 'comment', next: '@pop' }],
        [/./, 'comment'],
      ],
      interpolation: [
        [/\}\}/, { token: 'delimiter.bracket', next: '@pop' }],
        [/[\w.]+/, 'variable'],
        [/./, ''],
      ],
      script: [
        [/<\/script>/, { token: 'tag', next: '@pop' }],
        [/(import|export|from|const|let|var|function|return|if|else|for|while|class|interface|type|extends|implements)(?=\s)/, 'keyword'],
        [/(defineProps|defineEmits|defineExpose|defineOptions|defineSlots|defineModel|withDefaults)/, 'keyword.control.vue'],
        [/(ref|reactive|computed|watch|watchEffect|onMounted|onUnmounted|toRef|toRefs)/, 'support.function.vue'],
        [/"[^"]*"/, 'string'],
        [/'[^']*'/, 'string'],
        [/`[^`]*`/, 'string'],
        [/\/\/.*$/, 'comment'],
        [/\/\*/, { token: 'comment', next: '@comment' }],
        [/[{}()[\]]/, 'delimiter.bracket'],
        [/[<>]=?|[!=]=?=?|&&|\|\|/, 'operator'],
        [/\d+/, 'number'],
        [/[\w$]+/, 'identifier'],
        [/./, ''],
      ],
      comment: [
        [/\*\//, { token: 'comment', next: '@pop' }],
        [/./, 'comment'],
      ],
      style: [
        [/<\/style>/, { token: 'tag', next: '@pop' }],
        [/\/\*/, { token: 'comment', next: '@cssComment' }],
        [/[\w-]+(?=\s*:)/, 'attribute.name'],
        [/:/, 'delimiter'],
        [/[{}]/, 'delimiter.bracket'],
        [/"[^"]*"/, 'string'],
        [/'[^']*'/, 'string'],
        [/#[\da-fA-F]+/, 'number.hex'],
        [/\d+[\w%]*/, 'number'],
        [/[\w-]+/, 'attribute.value'],
        [/./, ''],
      ],
      cssComment: [
        [/\*\//, { token: 'comment', next: '@pop' }],
        [/./, 'comment'],
      ],
    },
  });

  // Set Vue language configuration
  // Note: Vue has different comment styles in different sections (template: HTML, script: JS, style: CSS)
  // We use HTML comments as default since template is common, but script comments are handled via custom keybinding
  monaco.languages.setLanguageConfiguration('vue', {
    comments: {
      // Use HTML comments for template sections (most common case for quick commenting)
      blockComment: ['<!--', '-->'],
    },
    brackets: [
      ['<!--', '-->'],
      ['<', '>'],
      ['{', '}'],
      ['[', ']'],
      ['(', ')'],
    ],
    autoClosingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '(', close: ')' },
      { open: '"', close: '"' },
      { open: "'", close: "'" },
      { open: '`', close: '`' },
      { open: '<', close: '>' },
      { open: '<!--', close: '-->' },
    ],
    surroundingPairs: [
      { open: '{', close: '}' },
      { open: '[', close: ']' },
      { open: '(', close: ')' },
      { open: '"', close: '"' },
      { open: "'", close: "'" },
      { open: '<', close: '>' },
    ],
  });

  // Define custom theme matching project CSS (Rust/Metal theme)
  monaco.editor.defineTheme('vue-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'keyword', foreground: 'e07048' },
      { token: 'keyword.control.vue', foreground: 'f08060', fontStyle: 'bold' },
      { token: 'support.function.vue', foreground: 'e07048' },
      { token: 'attribute.name.vue', foreground: 'e07048' },
      { token: 'variable', foreground: 'd0d4dc' },
      { token: 'tag', foreground: 'e07048' },
      { token: 'attribute.name', foreground: '9ca3b0' },
      { token: 'attribute.value', foreground: 'd0d4dc' },
      { token: 'string', foreground: 'd0d4dc' },
      { token: 'number', foreground: 'f08060' },
      { token: 'comment', foreground: '6b7280' },
      { token: 'delimiter.bracket', foreground: '9ca3b0' },
      { token: 'identifier', foreground: 'f0f2f5' },
    ],
    colors: {
      'editor.background': '#1a1b21',
      'editor.foreground': '#f0f2f5',
      'editor.lineHighlightBackground': '#252830',
      'editor.selectionBackground': '#e0704840',
      'editorCursor.foreground': '#e07048',
      'editorLineNumber.foreground': '#6b7280',
      'editorLineNumber.activeForeground': '#9ca3b0',
      'editorIndentGuide.background': '#252830',
      'editorIndentGuide.activeBackground': '#e0704840',
      'editor.inactiveSelectionBackground': '#e0704820',
    },
  });
}

// Apply scope decorations (called from watch and after mount)
function applyScopeDecorations(scopes: ScopeDecoration[] | undefined) {
  if (!editorInstance.value) return;
  const model = editorInstance.value.getModel();
  if (!model) return;

  if (!scopes || scopes.length === 0) {
    scopeDecorationIds = editorInstance.value.deltaDecorations(scopeDecorationIds, []);
    return;
  }

  const newDecorations: monaco.editor.IModelDeltaDecoration[] = scopes.map(scope => {
    const startPos = offsetToPosition(model, scope.start);
    const endPos = offsetToPosition(model, scope.end);
    const className = getScopeDecorationClass(scope.kindStr || scope.kind);

    return {
      range: new monaco.Range(startPos.lineNumber, startPos.column, endPos.lineNumber, endPos.column),
      options: {
        className,
        hoverMessage: { value: `**Scope:** ${scope.kindStr || scope.kind}` },
        isWholeLine: false,
        overviewRuler: {
          color: getOverviewRulerColor(scope.kind),
          position: monaco.editor.OverviewRulerLane.Right,
        },
      },
    };
  });

  scopeDecorationIds = editorInstance.value.deltaDecorations(scopeDecorationIds, newDecorations);
}

onMounted(() => {
  if (!containerRef.value) return;

  configureMonaco();

  editorInstance.value = monaco.editor.create(containerRef.value, {
    value: props.modelValue,
    language: props.language,
    theme: 'vue-dark',
    fontSize: 14,
    fontFamily: "'JetBrains Mono', monospace",
    minimap: { enabled: false },
    lineNumbers: 'on',
    scrollBeyondLastLine: false,
    padding: { top: 16 },
    automaticLayout: true,
    quickSuggestions: !props.readOnly,
    suggestOnTriggerCharacters: !props.readOnly,
    readOnly: props.readOnly ?? false,
    domReadOnly: props.readOnly ?? false,
  });

  editorInstance.value.onDidChangeModelContent(() => {
    const value = editorInstance.value?.getValue() || '';
    emit('update:modelValue', value);
  });

  // Custom comment toggle for Vue files - context-aware comments
  if (props.language === 'vue') {
    editorInstance.value.addAction({
      id: 'vue-toggle-line-comment',
      label: 'Toggle Line Comment (Vue-aware)',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Slash],
      run: (editor) => {
        const model = editor.getModel();
        const selection = editor.getSelection();
        if (!model || !selection) return;

        const content = model.getValue();
        const lineNumber = selection.startLineNumber;
        const lineContent = model.getLineContent(lineNumber);

        // Determine which section we're in by scanning backwards for section tags
        const beforeCursor = content.substring(0, model.getOffsetAt({ lineNumber, column: 1 }));
        // Use string concatenation to avoid Vue template parser interpreting these as tags
        const scriptOpen = '<' + 'script';
        const scriptClose = '</' + 'script>';
        const templateOpen = '<' + 'template';
        const styleOpen = '<' + 'style';
        const styleClose = '</' + 'style>';
        const isInScript = (beforeCursor.lastIndexOf(scriptOpen) > beforeCursor.lastIndexOf(scriptClose)) &&
                          (beforeCursor.lastIndexOf(scriptOpen) > beforeCursor.lastIndexOf(templateOpen));
        const isInStyle = (beforeCursor.lastIndexOf(styleOpen) > beforeCursor.lastIndexOf(styleClose)) &&
                         (beforeCursor.lastIndexOf(styleOpen) > beforeCursor.lastIndexOf(scriptClose));

        // Apply appropriate comment style
        const trimmedLine = lineContent.trim();
        let newLine: string;

        if (isInScript) {
          // JS-style comments for script section
          if (trimmedLine.startsWith('//')) {
            // Remove comment
            newLine = lineContent.replace(/^(\s*)\/\/\s?/, '$1');
          } else {
            // Add comment
            const leadingWhitespace = lineContent.match(/^(\s*)/)?.[1] || '';
            newLine = leadingWhitespace + '// ' + lineContent.trimStart();
          }
        } else if (isInStyle) {
          // CSS-style comments for style section
          if (trimmedLine.startsWith('/*') && trimmedLine.endsWith('*/')) {
            newLine = lineContent.replace(/^(\s*)\/\*\s?/, '$1').replace(/\s?\*\/$/, '');
          } else {
            const leadingWhitespace = lineContent.match(/^(\s*)/)?.[1] || '';
            newLine = leadingWhitespace + '/* ' + lineContent.trimStart() + ' */';
          }
        } else {
          // HTML-style comments for template section
          if (trimmedLine.startsWith('<!--') && trimmedLine.endsWith('-->')) {
            newLine = lineContent.replace(/^(\s*)<!--\s?/, '$1').replace(/\s?-->$/, '');
          } else {
            const leadingWhitespace = lineContent.match(/^(\s*)/)?.[1] || '';
            newLine = leadingWhitespace + '<!-- ' + lineContent.trimStart() + ' -->';
          }
        }

        // Apply the edit
        editor.executeEdits('vue-comment', [{
          range: new monaco.Range(lineNumber, 1, lineNumber, lineContent.length + 1),
          text: newLine,
        }]);
      },
    });
  }

  // Apply scopes if they were already set before mount
  if (props.scopes && props.scopes.length > 0) {
    applyScopeDecorations(props.scopes);
  }

  // Apply diagnostics if they were already set before mount
  if (props.diagnostics && props.diagnostics.length > 0) {
    applyDiagnostics(props.diagnostics);
  }
});

onUnmounted(() => {
  editorInstance.value?.dispose();
});

watch(() => props.modelValue, (newValue) => {
  if (editorInstance.value && editorInstance.value.getValue() !== newValue) {
    editorInstance.value.setValue(newValue);
  }
});

watch(() => props.language, (newLanguage) => {
  if (editorInstance.value) {
    const model = editorInstance.value.getModel();
    if (model) {
      monaco.editor.setModelLanguage(model, newLanguage);
    }
  }
});

// Apply diagnostics to editor
function applyDiagnostics(diagnostics: Diagnostic[] | undefined) {
  if (!editorInstance.value) return;
  const model = editorInstance.value.getModel();
  if (!model) return;

  if (!diagnostics || diagnostics.length === 0) {
    monaco.editor.setModelMarkers(model, 'vize', []);
    return;
  }

  const markers: monaco.editor.IMarkerData[] = diagnostics.map(d => ({
    severity: d.severity === 'error'
      ? monaco.MarkerSeverity.Error
      : d.severity === 'warning'
        ? monaco.MarkerSeverity.Warning
        : monaco.MarkerSeverity.Info,
    message: d.message,
    startLineNumber: d.startLine,
    startColumn: d.startColumn,
    endLineNumber: d.endLine ?? d.startLine,
    endColumn: d.endColumn ?? d.startColumn + 1,
  }));

  monaco.editor.setModelMarkers(model, 'vize', markers);
}

// Update diagnostics markers
watch(() => props.diagnostics, (diagnostics) => {
  applyDiagnostics(diagnostics);
}, { immediate: true });

// Scope decoration IDs
let scopeDecorationIds: string[] = [];

// Scope kind to CSS class mapping (O(1) lookup for exact matches)
const SCOPE_CLASS_MAP: Record<string, string> = {
  'setup': 'scope-decoration-setup',
  'plain': 'scope-decoration-plain',
  'extern': 'scope-decoration-extern',
  'extmod': 'scope-decoration-extern',
  'vue': 'scope-decoration-vue',
  'universal': 'scope-decoration-universal',
  'server': 'scope-decoration-server',
  'client': 'scope-decoration-client',
  'vfor': 'scope-decoration-vFor',
  'v-for': 'scope-decoration-vFor',
  'vslot': 'scope-decoration-vSlot',
  'v-slot': 'scope-decoration-vSlot',
  'function': 'scope-decoration-function',
  'arrowfunction': 'scope-decoration-function',
  'block': 'scope-decoration-block',
  'mod': 'scope-decoration-mod',
  'closure': 'scope-decoration-closure',
  'event': 'scope-decoration-event',
  'callback': 'scope-decoration-callback',
};

// Get scope decoration class based on kind
function getScopeDecorationClass(kind: string): string {
  const kindLower = kind.toLowerCase();
  // Fast path: exact match
  const exact = SCOPE_CLASS_MAP[kindLower];
  if (exact) return exact;
  // Fallback: pattern matching for compound kinds
  if (kindLower.includes('clientonly') || kindLower.includes('mounted')) return 'scope-decoration-client';
  if (kindLower.includes('computed')) return 'scope-decoration-computed';
  if (kindLower.includes('watch')) return 'scope-decoration-watch';
  return 'scope-decoration-default';
}

// Convert offset to position
function offsetToPosition(model: monaco.editor.ITextModel, offset: number): monaco.IPosition {
  const content = model.getValue();
  const safeOffset = Math.min(offset, content.length);
  let line = 1;
  let column = 1;

  for (let i = 0; i < safeOffset; i++) {
    if (content[i] === '\n') {
      line++;
      column = 1;
    } else {
      column++;
    }
  }

  return { lineNumber: line, column };
}

// Update scope decorations when scopes prop changes
watch(() => props.scopes, (scopes) => {
  applyScopeDecorations(scopes);
}, { immediate: true });

// Overview ruler color mapping (O(1) lookup)
const RULER_COLOR_MAP: Record<string, string> = {
  'setup': '#22c55e40',
  'vue': '#42b88340',
  'client': '#f97316a0',
  'server': '#3b82f6a0',
  'universal': '#8b5cf640',
  'vfor': '#a78bfa40',
  'v-for': '#a78bfa40',
  'vslot': '#f472b640',
  'v-slot': '#f472b640',
  'closure': '#fbbf2440',
  'block': '#94a3b830',
  'event': '#f472b640',
  'callback': '#fb923c40',
};

// Get overview ruler color based on scope kind
function getOverviewRulerColor(kind: string): string {
  const kindLower = kind.toLowerCase();
  return RULER_COLOR_MAP[kindLower] || '#9ca3b020';
}
</script>

<template>
  <div ref="containerRef" class="monaco-container"></div>
</template>

<style scoped>
.monaco-container {
  width: 100%;
  height: 100%;
}
</style>

<!-- Global styles for Monaco decorations (must not be scoped) -->
<style>
/* Scope decoration styles - using abbreviated names with better visibility */
.scope-decoration-setup {
  background: rgba(34, 197, 94, 0.2);
  border-left: 3px solid #22c55e;
}

.scope-decoration-plain {
  background: rgba(251, 191, 36, 0.2);
  border-left: 3px solid #fbbf24;
}

.scope-decoration-extern {
  background: rgba(96, 165, 250, 0.2);
  border-left: 3px solid #60a5fa;
}

.scope-decoration-vue {
  background: rgba(66, 184, 131, 0.25);
  border-left: 3px solid #42b883;
}

.scope-decoration-client {
  background: rgba(249, 115, 22, 0.25);
  border-left: 3px solid #f97316;
}

.scope-decoration-server {
  background: rgba(59, 130, 246, 0.25);
  border-left: 3px solid #3b82f6;
}

.scope-decoration-universal {
  background: rgba(139, 92, 246, 0.2);
  border-left: 3px solid #8b5cf6;
}

.scope-decoration-vFor {
  background: rgba(167, 139, 250, 0.2);
  border-left: 3px solid #a78bfa;
}

.scope-decoration-vSlot {
  background: rgba(244, 114, 182, 0.2);
  border-left: 3px solid #f472b6;
}

.scope-decoration-function {
  background: rgba(45, 212, 191, 0.15);
  border-left: 3px solid #2dd4bf;
}

.scope-decoration-closure {
  background: rgba(251, 191, 36, 0.15);
  border-left: 3px solid #fbbf24;
}

.scope-decoration-block {
  background: rgba(148, 163, 184, 0.1);
  border-left: 3px solid #94a3b8;
}

.scope-decoration-event {
  background: rgba(244, 114, 182, 0.15);
  border-left: 3px solid #f472b6;
}

.scope-decoration-callback {
  background: rgba(251, 146, 60, 0.15);
  border-left: 3px solid #fb923c;
}

.scope-decoration-computed {
  background: rgba(251, 146, 60, 0.2);
  border-left: 3px solid #fb923c;
}

.scope-decoration-watch {
  background: rgba(99, 102, 241, 0.2);
  border-left: 3px solid #6366f1;
}

.scope-decoration-mod {
  background: rgba(156, 163, 175, 0.1);
  border-left: 3px solid #9ca3af;
}

.scope-decoration-default {
  background: rgba(156, 163, 175, 0.1);
  border-left: 3px solid #6b7280;
}
</style>
