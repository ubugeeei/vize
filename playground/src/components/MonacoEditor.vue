<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, shallowRef } from 'vue';
import * as monaco from 'monaco-editor';

// Script tag attributes
const SCRIPT_TAG_ATTRS = [
  { label: 'setup', insertText: 'setup', detail: 'Enable <script setup> syntax' },
  { label: 'vapor', insertText: 'vapor', detail: 'Enable Vapor mode compilation' },
  { label: 'lang="ts"', insertText: 'lang="ts"', detail: 'Use TypeScript' },
  { label: 'lang="tsx"', insertText: 'lang="tsx"', detail: 'Use TSX' },
  { label: 'generic', insertText: 'generic="${1:T}"', detail: 'Define generic type parameters' },
];

// Template tag attributes
const TEMPLATE_TAG_ATTRS = [
  { label: 'lang="pug"', insertText: 'lang="pug"', detail: 'Use Pug template syntax' },
];

// Style tag attributes
const STYLE_TAG_ATTRS = [
  { label: 'scoped', insertText: 'scoped', detail: 'Scope styles to this component' },
  { label: 'module', insertText: 'module', detail: 'Enable CSS modules' },
  { label: 'lang="scss"', insertText: 'lang="scss"', detail: 'Use SCSS' },
  { label: 'lang="less"', insertText: 'lang="less"', detail: 'Use Less' },
];

// Vue compiler macros for completion
const VUE_COMPILER_MACROS = [
  { label: 'defineProps', insertText: 'defineProps<${1:Props}>()', detail: 'Define component props' },
  { label: 'defineEmits', insertText: 'defineEmits<${1:Emits}>()', detail: 'Define component emits' },
  { label: 'defineExpose', insertText: 'defineExpose({ $1 })', detail: 'Expose component methods' },
  { label: 'defineOptions', insertText: 'defineOptions({ $1 })', detail: 'Define component options' },
  { label: 'defineSlots', insertText: 'defineSlots<${1:Slots}>()', detail: 'Define typed slots' },
  { label: 'defineModel', insertText: 'defineModel<${1:T}>(${2})', detail: 'Define v-model binding' },
  { label: 'withDefaults', insertText: 'withDefaults(defineProps<${1:Props}>(), {\n  $2\n})', detail: 'Props with defaults' },
];

// Vue reactivity APIs
const VUE_REACTIVITY_APIS = [
  { label: 'ref', insertText: 'ref($1)', detail: 'Create a reactive reference' },
  { label: 'reactive', insertText: 'reactive({ $1 })', detail: 'Create a reactive object' },
  { label: 'computed', insertText: 'computed(() => $1)', detail: 'Create a computed value' },
  { label: 'watch', insertText: 'watch($1, ($2) => {\n  $3\n})', detail: 'Watch reactive source' },
  { label: 'watchEffect', insertText: 'watchEffect(() => {\n  $1\n})', detail: 'Run effect immediately' },
  { label: 'toRef', insertText: 'toRef($1, \'$2\')', detail: 'Create ref from reactive property' },
  { label: 'toRefs', insertText: 'toRefs($1)', detail: 'Convert reactive to refs' },
];

const props = defineProps<{
  modelValue: string;
  language: string;
}>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void;
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
        [/\{\{/, { token: 'delimiter.bracket', next: '@interpolation' }],
        [/<\/?[\w-]+/, { token: 'tag', next: '@tag' }],
        [/./, ''],
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
    },
  });

  // Set Vue language configuration
  monaco.languages.setLanguageConfiguration('vue', {
    comments: {
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

  // Register completion provider for Vue compiler macros and reactivity APIs
  monaco.languages.registerCompletionItemProvider('vue', {
    triggerCharacters: ['d', 'r', 'c', 'w', 't'],
    provideCompletionItems: (model, position) => {
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      const textUntilPosition = model.getValueInRange({
        startLineNumber: 1,
        startColumn: 1,
        endLineNumber: position.lineNumber,
        endColumn: position.column,
      });

      const isInScriptSetup = /<script[^>]*setup[^>]*>/.test(textUntilPosition) &&
        !/<\/script>/.test(textUntilPosition.split(/<script[^>]*setup[^>]*>/)[1] || '');

      if (!isInScriptSetup) {
        return { suggestions: [] };
      }

      const suggestions = [
        ...VUE_COMPILER_MACROS.map(macro => ({
          label: macro.label,
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: macro.insertText,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: macro.detail,
          range,
        })),
        ...VUE_REACTIVITY_APIS.map(api => ({
          label: api.label,
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: api.insertText,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: api.detail,
          range,
        })),
      ];

      return { suggestions };
    },
  });

  // Register completion provider for SFC tag attributes
  monaco.languages.registerCompletionItemProvider('vue', {
    triggerCharacters: [' '],
    provideCompletionItems: (model, position) => {
      const lineContent = model.getLineContent(position.lineNumber);
      const textBeforeCursor = lineContent.substring(0, position.column - 1);

      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      const scriptTagMatch = textBeforeCursor.match(/<script\s+(?![^>]*>)/);
      const templateTagMatch = textBeforeCursor.match(/<template\s+(?![^>]*>)/);
      const styleTagMatch = textBeforeCursor.match(/<style\s+(?![^>]*>)/);

      let attrs: typeof SCRIPT_TAG_ATTRS = [];

      if (scriptTagMatch) {
        const usedAttrs: string[] = textBeforeCursor.match(/\b(setup|vapor|lang|generic)\b/g) || [];
        attrs = SCRIPT_TAG_ATTRS.filter(attr => {
          const attrName = attr.label.split('=')[0].split('"')[0];
          return !usedAttrs.includes(attrName);
        });
      } else if (templateTagMatch) {
        const usedAttrs: string[] = textBeforeCursor.match(/\b(lang)\b/g) || [];
        attrs = TEMPLATE_TAG_ATTRS.filter(attr => {
          const attrName = attr.label.split('=')[0].split('"')[0];
          return !usedAttrs.includes(attrName);
        });
      } else if (styleTagMatch) {
        const usedAttrs: string[] = textBeforeCursor.match(/\b(scoped|module|lang)\b/g) || [];
        attrs = STYLE_TAG_ATTRS.filter(attr => {
          const attrName = attr.label.split('=')[0].split('"')[0];
          return !usedAttrs.includes(attrName);
        });
      }

      if (attrs.length === 0) {
        return { suggestions: [] };
      }

      const suggestions = attrs.map(attr => ({
        label: attr.label,
        kind: monaco.languages.CompletionItemKind.Property,
        insertText: attr.insertText,
        insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
        detail: attr.detail,
        range,
      }));

      return { suggestions };
    },
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
    quickSuggestions: true,
    suggestOnTriggerCharacters: true,
  });

  editorInstance.value.onDidChangeModelContent(() => {
    const value = editorInstance.value?.getValue() || '';
    emit('update:modelValue', value);
  });
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
