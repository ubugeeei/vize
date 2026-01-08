<script setup lang="ts">
import { ref, watch, onMounted } from 'vue';
import { createHighlighter, type Highlighter, type ThemeRegistration } from 'shiki';

const props = defineProps<{
  code: string;
  language: 'javascript' | 'json' | 'css' | 'html' | 'typescript';
  showLineNumbers?: boolean;
}>();

// Custom theme matching project's Rust/Metal design
const vizeTheme: ThemeRegistration = {
  name: 'vize-dark',
  type: 'dark',
  colors: {
    'editor.background': '#1a1b21',
    'editor.foreground': '#f0f2f5',
  },
  tokenColors: [
    { scope: ['keyword', 'storage.type', 'storage.modifier'], settings: { foreground: '#e07048' } },
    { scope: ['entity.name.function', 'support.function'], settings: { foreground: '#f08060' } },
    { scope: ['entity.name.tag', 'punctuation.definition.tag'], settings: { foreground: '#e07048' } },
    { scope: ['entity.other.attribute-name'], settings: { foreground: '#9ca3b0' } },
    { scope: ['string', 'string.quoted'], settings: { foreground: '#d0d4dc' } },
    { scope: ['constant.numeric', 'constant.language'], settings: { foreground: '#f08060' } },
    { scope: ['variable', 'variable.other'], settings: { foreground: '#f0f2f5' } },
    { scope: ['comment', 'punctuation.definition.comment'], settings: { foreground: '#6b7280' } },
    { scope: ['punctuation', 'meta.brace'], settings: { foreground: '#9ca3b0' } },
    { scope: ['entity.name.type', 'support.type'], settings: { foreground: '#d0d4dc' } },
    { scope: ['meta.property-name', 'support.type.property-name'], settings: { foreground: '#e07048' } },
    { scope: ['meta.property-value', 'support.constant.property-value'], settings: { foreground: '#d0d4dc' } },
  ],
};

const highlightedCode = ref('');
let highlighter: Highlighter | null = null;

async function initHighlighter() {
  if (!highlighter) {
    highlighter = await createHighlighter({
      themes: [vizeTheme],
      langs: ['javascript', 'json', 'css', 'html', 'typescript'],
    });
  }
  return highlighter;
}

async function highlight() {
  const hl = await initHighlighter();
  const html = hl.codeToHtml(props.code, {
    lang: props.language,
    theme: 'vize-dark',
  });
  highlightedCode.value = html;
}

onMounted(highlight);
watch(() => [props.code, props.language], highlight);
</script>

<template>
  <div class="code-highlight" :class="{ 'with-line-numbers': showLineNumbers }">
    <div v-html="highlightedCode"></div>
  </div>
</template>

<style scoped>
.code-highlight {
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  line-height: 1.6;
  border-radius: 12px;
  overflow: auto;
}

.code-highlight :deep(pre) {
  margin: 0;
  padding: 16px !important;
  background: #1a1b21 !important;
  border-radius: 12px;
  overflow: auto;
  white-space: pre;
}

.code-highlight :deep(code) {
  font-family: inherit;
}

.code-highlight.with-line-numbers :deep(.line) {
  display: flex;
}

.code-highlight.with-line-numbers :deep(.line::before) {
  content: counter(line);
  counter-increment: line;
  display: inline-block;
  width: 30px;
  color: #71717a;
  user-select: none;
  text-align: right;
  margin-right: 16px;
  font-size: 12px;
}

.code-highlight.with-line-numbers :deep(code) {
  counter-reset: line;
}
</style>
