<script setup lang="ts">
import type { TokenCategory, DesignToken, TokenUsageMap } from '../../api'
import TokenCard from './TokenCard.vue'

const props = withDefaults(defineProps<{
  category: TokenCategory
  level?: number
  parentPath?: string
  usageMap?: TokenUsageMap
}>(), {
  usageMap: () => ({}),
})

const emit = defineEmits<{
  edit: [path: string, token: DesignToken]
  delete: [path: string, token: DesignToken]
  showUsage: [tokenPath: string]
}>()

const headingLevel = Math.min((props.level ?? 2), 6)

function getCategoryPath(): string {
  const catKey = props.category.name.toLowerCase().replace(/\s+/g, '-')
  return props.parentPath ? `${props.parentPath}.${catKey}` : catKey
}

function getTokenPath(name: string): string {
  return `${getCategoryPath()}.${name}`
}

function getUsageCount(name: string): number {
  const tokenPath = getTokenPath(name)
  const entries = props.usageMap[tokenPath]
  if (!entries) return 0
  return entries.reduce((sum, entry) => sum + entry.matches.length, 0)
}
</script>

<template>
  <div class="token-category" :class="{ 'token-subcategory': level && level > 2 }">
    <component :is="'h' + headingLevel" class="category-title" :class="'category-title--h' + headingLevel">
      {{ category.name }}
    </component>

    <div v-if="Object.keys(category.tokens).length > 0" class="tokens-grid">
      <TokenCard
        v-for="(token, name) in category.tokens"
        :key="name"
        :name="String(name)"
        :token="token"
        :category-path="getCategoryPath()"
        :usage-count="getUsageCount(String(name))"
        @edit="emit('edit', getTokenPath(String(name)), token)"
        @delete="emit('delete', getTokenPath(String(name)), token)"
        @show-usage="emit('showUsage', getTokenPath(String(name)))"
      />
    </div>

    <template v-if="category.subcategories">
      <TokenCategorySection
        v-for="sub in category.subcategories"
        :key="sub.name"
        :category="sub"
        :level="(level ?? 2) + 1"
        :parent-path="getCategoryPath()"
        :usage-map="usageMap"
        @edit="(path, token) => emit('edit', path, token)"
        @delete="(path, token) => emit('delete', path, token)"
        @show-usage="(tokenPath) => emit('showUsage', tokenPath)"
      />
    </template>
  </div>
</template>

<script lang="ts">
// Recursive component self-reference
import { defineComponent } from 'vue'
export default defineComponent({
  name: 'TokenCategorySection',
})
</script>

<style scoped>
.token-category {
  margin-bottom: 2.5rem;
}

.token-subcategory {
  margin-top: 1.5rem;
  margin-left: 1rem;
  margin-bottom: 0;
}

.category-title {
  font-weight: 600;
  margin-bottom: 1rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--musea-border);
}

.category-title--h2 {
  font-size: 1.125rem;
}

.category-title--h3 {
  font-size: 0.9375rem;
  color: var(--musea-text-secondary);
  border-bottom: none;
  padding-bottom: 0;
}

.category-title--h4,
.category-title--h5,
.category-title--h6 {
  font-size: 0.875rem;
  color: var(--musea-text-muted);
  border-bottom: none;
  padding-bottom: 0;
}

.tokens-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 1rem;
}
</style>
