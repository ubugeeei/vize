/**
 * Croquis Playground Preset
 *
 * This preset demonstrates the Vue SFC analysis capabilities:
 * - Script (Options API) and Script Setup blocks
 * - Props/emits type definitions and exports
 * - Lifecycle hooks, watchers, and reactive state
 * - Template directives (v-for, v-slot)
 * - Scoped styles with v-bind in CSS
 *
 * Note: This file is separate from the Vue component to avoid
 * linting issues with embedded Vue code in template literals.
 */

export const ANALYSIS_PRESET = `<script lang="ts">
// Non-script-setup block (Options API compatible)
import axios from 'axios'
import { formatDate } from '@/utils/date'
import type { User } from '@/types'

export default {
  name: 'DemoComponent',
  inheritAttrs: false,
}
</script>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
// External module imports
import lodash from 'lodash'
import dayjs from 'dayjs'

// Type exports (hoisted to module level - VALID in script setup)
export type ComponentProps = {
  title: string
  count?: number
}

export interface ComponentEmits {
  (e: 'update', value: number): void
  (e: 'close'): void
}

// Props and emits using the exported types
const props = defineProps<ComponentProps>()

// Emits declaration
const emit = defineEmits<ComponentEmits>()

// Example of invalid exports (uncomment to see error detection)
// export const INVALID_CONST = 'error'
// export function invalidFunction() {}

// Reactive refs
const counter = ref(0)
const doubled = computed(() => counter.value * 2)
const message = ref('Hello Vue!')
const windowWidth = ref(0)

// Watchers
watch(counter, (newVal) => {
  console.log('Counter changed:', newVal)
})

// Client-only lifecycle hooks (SSR safe)
onMounted(() => {
  // This code only runs on the client
  windowWidth.value = window.innerWidth
  console.log('Component mounted on client')
})

onUnmounted(() => {
  // Cleanup on client
  console.log('Component unmounted')
})

// Methods
function increment() {
  counter.value++
  emit('update', counter.value)
}

function reset() {
  counter.value = 0
}

// Array operations with scoped callbacks
const items = ref([1, 2, 3, 4, 5])
const evenItems = computed(() => items.value.filter((item) => item % 2 === 0))
const mappedItems = computed(() => items.value.map((item) => item * 2))
</script>

<template>
  <div class="container">
    <h1>{{ props.title }}</h1>
    <p class="message">{{ message }}</p>
    <p class="width">Window width: {{ windowWidth }}px</p>
    <div class="counter">
      <span>Count: {{ counter }}</span>
      <span>Doubled: {{ doubled }}</span>
    </div>
    <div class="actions">
      <button @click="increment">+1</button>
      <button @click="reset">Reset</button>
      <button @click="(e) => console.log('clicked', e)">Log Event</button>
    </div>

    <!-- v-for scope example -->
    <ul>
      <li v-for="(item, index) in evenItems" :key="item">
        {{ index }}: {{ item }}
      </li>
    </ul>

    <!-- scoped slot example -->
    <MyList v-slot="{ item, selected }">
      <span :class="{ active: selected }">{{ item.name }}</span>
    </MyList>

    <!-- named scoped slot -->
    <DataTable>
      <template #header="{ columns }">
        <th v-for="col in columns" :key="col.id">{{ col.label }}</th>
      </template>
      <template #row="{ row, index }">
        <td>{{ index }}: {{ row.name }}</td>
      </template>
    </DataTable>
  </div>
</template>

<style scoped>
.container {
  padding: 20px;
  font-family: system-ui, sans-serif;
}

.message {
  color: v-bind('counter > 5 ? "red" : "green"');
}

.width {
  color: #666;
  font-size: 14px;
}

.counter {
  display: flex;
  gap: 16px;
  margin: 16px 0;
}

.actions {
  display: flex;
  gap: 8px;
}

button {
  padding: 8px 16px;
  border-radius: 4px;
  border: 1px solid #ccc;
  cursor: pointer;
}
</style>
`;
