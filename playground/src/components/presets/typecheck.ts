/**
 * TypeCheck Playground Presets
 *
 * Two presets demonstrating TypeScript type checking:
 *
 * 1. TYPECHECK_PRESET - Untyped version with issues:
 *    - defineProps without type definition
 *    - defineEmits without type definition
 *    Shows the warnings that would be generated
 *
 * 2. TYPECHECK_TYPED_PRESET - Properly typed version:
 *    - Props with interface type
 *    - Emits with typed function signatures
 *    Shows the recommended approach
 *
 * Note: This file is separate from the Vue component to avoid
 * linting issues with embedded Vue code in template literals.
 */

export const TYPECHECK_PRESET = `<script setup lang="ts">
import { ref } from 'vue'

// Props without type definition - triggers warning
const props = defineProps()

// Emits without type definition - triggers warning
const emit = defineEmits()

const count = ref(0)
const message = ref('Hello')

function increment() {
  count.value++
}
</script>

<template>
  <div class="container">
    <h1>{{ message }}</h1>
    <p>Count: {{ count }}</p>
    <button @click="increment">+1</button>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
}
</style>
`;

export const TYPECHECK_TYPED_PRESET = `<script setup lang="ts">
import { ref } from 'vue'

// Props with type definition - no warning
interface Props {
  title: string
  count?: number
}
const props = defineProps<Props>()

// Emits with type definition - no warning
interface Emits {
  (e: 'update', value: number): void
  (e: 'reset'): void
}
const emit = defineEmits<Emits>()

const localCount = ref(props.count ?? 0)
const message = ref('Hello')

function increment() {
  localCount.value++
  emit('update', localCount.value)
}

function reset() {
  localCount.value = 0
  emit('reset')
}
</script>

<template>
  <div class="container">
    <h1>{{ props.title }}: {{ message }}</h1>
    <p>Count: {{ localCount }}</p>
    <button @click="increment">+1</button>
    <button @click="reset">Reset</button>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
}
button {
  margin: 0 4px;
}
</style>
`;
