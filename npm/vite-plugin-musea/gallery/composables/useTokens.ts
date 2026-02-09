import { ref, computed } from 'vue'
import type { DesignToken, TokenCategory, TokensMeta } from '../api'
import { fetchTokens, createToken, updateToken, deleteToken } from '../api'

const categories = ref<TokenCategory[]>([])
const tokenMap = ref<Record<string, DesignToken>>({})
const meta = ref<TokensMeta>({ filePath: '', tokenCount: 0, primitiveCount: 0, semanticCount: 0, componentCount: 0 })
const loading = ref(false)
const error = ref<string | null>(null)
const activeTab = ref<'all' | 'primitive' | 'semantic' | 'component'>('all')
const filter = ref('')

let loaded = false

function filterCategoryByTier(cat: TokenCategory, tier: 'primitive' | 'semantic' | 'component'): TokenCategory | null {
  const matchingTokens: Record<string, DesignToken> = {}
  for (const [name, token] of Object.entries(cat.tokens)) {
    if (token.$tier === tier) {
      matchingTokens[name] = token
    }
  }

  const matchingSubs = (cat.subcategories ?? [])
    .map(sub => filterCategoryByTier(sub, tier))
    .filter((sub): sub is TokenCategory => sub !== null)

  if (Object.keys(matchingTokens).length > 0 || matchingSubs.length > 0) {
    return {
      name: cat.name,
      tokens: matchingTokens,
      subcategories: matchingSubs.length > 0 ? matchingSubs : undefined,
    }
  }
  return null
}

function filterCategoryByQuery(cat: TokenCategory, query: string): TokenCategory | null {
  const matchingTokens: Record<string, DesignToken> = {}
  for (const [name, token] of Object.entries(cat.tokens)) {
    if (
      name.toLowerCase().includes(query) ||
      String(token.value).toLowerCase().includes(query) ||
      (token.description ?? '').toLowerCase().includes(query) ||
      (token.$reference ?? '').toLowerCase().includes(query)
    ) {
      matchingTokens[name] = token
    }
  }

  const matchingSubs = (cat.subcategories ?? [])
    .map(sub => filterCategoryByQuery(sub, query))
    .filter((sub): sub is TokenCategory => sub !== null)

  if (Object.keys(matchingTokens).length > 0 || matchingSubs.length > 0) {
    return {
      name: cat.name,
      tokens: matchingTokens,
      subcategories: matchingSubs.length > 0 ? matchingSubs : undefined,
    }
  }
  return null
}

export function useTokens() {
  const filteredCategories = computed(() => {
    let result = categories.value

    // Filter by tier
    if (activeTab.value !== 'all') {
      result = result
        .map(cat => filterCategoryByTier(cat, activeTab.value as 'primitive' | 'semantic' | 'component'))
        .filter((cat): cat is TokenCategory => cat !== null)
    }

    // Filter by search query
    if (filter.value) {
      const q = filter.value.toLowerCase()
      result = result
        .map(cat => filterCategoryByQuery(cat, q))
        .filter((cat): cat is TokenCategory => cat !== null)
    }

    return result
  })

  const primitiveTokenPaths = computed(() => {
    return Object.entries(tokenMap.value)
      .filter(([, token]) => token.$tier === 'primitive')
      .map(([path]) => path)
  })

  const referenceableTokenPaths = computed(() => {
    return Object.entries(tokenMap.value)
      .filter(([, token]) => token.$tier === 'primitive' || token.$tier === 'semantic')
      .map(([path]) => path)
  })

  async function load() {
    if (loaded) return
    loading.value = true
    error.value = null
    try {
      const data = await fetchTokens()
      categories.value = data.categories
      tokenMap.value = data.tokenMap
      meta.value = data.meta
      loaded = true
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function reload() {
    loaded = false
    await load()
  }

  async function addToken(tokenPath: string, token: Omit<DesignToken, '$resolvedValue'>) {
    const result = await createToken(tokenPath, token)
    categories.value = result.categories
    tokenMap.value = result.tokenMap
  }

  async function editToken(tokenPath: string, token: Omit<DesignToken, '$resolvedValue'>) {
    const result = await updateToken(tokenPath, token)
    categories.value = result.categories
    tokenMap.value = result.tokenMap
  }

  async function removeToken(tokenPath: string) {
    const result = await deleteToken(tokenPath)
    categories.value = result.categories
    tokenMap.value = result.tokenMap
    return result.dependentsWarning
  }

  function resolveReference(ref: string): string | number | undefined {
    return tokenMap.value[ref]?.$resolvedValue ?? tokenMap.value[ref]?.value
  }

  return {
    categories,
    tokenMap,
    meta,
    loading,
    error,
    activeTab,
    filter,
    filteredCategories,
    primitiveTokenPaths,
    referenceableTokenPaths,
    load,
    reload,
    addToken,
    editToken,
    removeToken,
    resolveReference,
  }
}
