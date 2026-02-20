<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import type { ArtVariant } from '../../src/types.js'

const props = defineProps<{
  variants: ArtVariant[]
  selectedVariant: string
}>()

const emit = defineEmits<{
  (e: 'select', variantName: string): void
}>()

const tabsRef = ref<HTMLElement | null>(null)
const showLeftArrow = ref(false)
const showRightArrow = ref(false)

const defaultVariant = computed(() =>
  props.variants.find(v => v.isDefault)?.name || props.variants[0]?.name
)

const checkScrollButtons = () => {
  if (!tabsRef.value) return
  const el = tabsRef.value
  showLeftArrow.value = el.scrollLeft > 0
  showRightArrow.value = el.scrollLeft < el.scrollWidth - el.clientWidth - 1
}

const scroll = (direction: 'left' | 'right') => {
  if (!tabsRef.value) return
  const scrollAmount = 200
  tabsRef.value.scrollBy({
    left: direction === 'left' ? -scrollAmount : scrollAmount,
    behavior: 'smooth'
  })
}

onMounted(() => {
  checkScrollButtons()
  window.addEventListener('resize', checkScrollButtons)
})

onUnmounted(() => {
  window.removeEventListener('resize', checkScrollButtons)
})

watch(() => props.variants, checkScrollButtons)
</script>

<template>
  <div class="variant-tabs-container">
    <button
      v-if="showLeftArrow"
      type="button"
      class="scroll-btn scroll-btn--left"
      @click="scroll('left')"
      aria-label="Scroll left"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="15 18 9 12 15 6" />
      </svg>
    </button>

    <div
      ref="tabsRef"
      class="variant-tabs"
      @scroll="checkScrollButtons"
    >
      <button
        v-for="variant in variants"
        :key="variant.name"
        type="button"
        :class="['variant-tab', {
          'variant-tab--active': variant.name === selectedVariant,
          'variant-tab--default': variant.isDefault
        }]"
        @click="emit('select', variant.name)"
      >
        <span class="variant-tab-name">{{ variant.name }}</span>
        <span v-if="variant.isDefault" class="variant-tab-badge">Default</span>
      </button>
    </div>

    <button
      v-if="showRightArrow"
      type="button"
      class="scroll-btn scroll-btn--right"
      @click="scroll('right')"
      aria-label="Scroll right"
    >
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="9 18 15 12 9 6" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.variant-tabs-container {
  position: relative;
  display: flex;
  align-items: stretch;
  background: var(--musea-bg-secondary);
  border-bottom: 1px solid var(--musea-border);
  flex-shrink: 0;
}

.variant-tabs {
  display: flex;
  overflow-x: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
  flex: 1;
  gap: 0.125rem;
  padding: 0.125rem 0.25rem;
}

.variant-tabs::-webkit-scrollbar {
  display: none;
}

.variant-tab {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.25rem 0.5rem;
  background: transparent;
  border: 1px solid transparent;
  border-radius: 2px;
  color: var(--musea-text-muted);
  font-size: 0.625rem;
  font-weight: 500;
  white-space: nowrap;
  cursor: pointer;
  transition: all 0.15s ease;
}

.variant-tab:hover {
  background: var(--musea-bg-tertiary);
  color: var(--musea-text);
}

.variant-tab--active {
  background: var(--musea-accent-subtle);
  border-color: var(--musea-accent);
  color: var(--musea-accent);
}

.variant-tab--active:hover {
  background: var(--musea-accent-subtle);
}

.variant-tab-name {
  font-weight: 500;
}

.variant-tab-badge {
  font-size: 0.5rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  padding: 0.0625rem 0.25rem;
  border-radius: 2px;
  background: var(--musea-accent);
  color: white;
}

.variant-tab--active .variant-tab-badge {
  background: var(--musea-accent);
  color: white;
}

.scroll-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  background: var(--musea-bg-secondary);
  border: none;
  color: var(--musea-text-muted);
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s;
}

.scroll-btn:hover {
  background: var(--musea-bg-tertiary);
  color: var(--musea-text);
}

.scroll-btn svg {
  width: 12px;
  height: 12px;
}

.scroll-btn--left {
  border-right: 1px solid var(--musea-border);
}

.scroll-btn--right {
  border-left: 1px solid var(--musea-border);
}
</style>
