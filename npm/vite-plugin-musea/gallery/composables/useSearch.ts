import { ref, computed } from 'vue'
import type { ArtFileInfo } from '../../src/types.js'

const query = ref('')

export function useSearch(arts: { value: ArtFileInfo[] }) {
  const results = computed(() => {
    const q = query.value.toLowerCase().trim()
    if (!q) return arts.value

    return arts.value.filter(art => {
      const title = art.metadata.title.toLowerCase()
      const desc = (art.metadata.description ?? '').toLowerCase()
      const category = (art.metadata.category ?? '').toLowerCase()
      const tags = art.metadata.tags.map(t => t.toLowerCase())

      return (
        title.includes(q) ||
        desc.includes(q) ||
        category.includes(q) ||
        tags.some(t => t.includes(q)) ||
        art.variants.some(v => v.name.toLowerCase().includes(q))
      )
    })
  })

  function setQuery(q: string) {
    query.value = q
  }

  function clear() {
    query.value = ''
  }

  return {
    query,
    results,
    setQuery,
    clear,
  }
}
