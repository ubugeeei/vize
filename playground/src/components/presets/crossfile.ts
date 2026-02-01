/**
 * CrossFile Playground Presets
 *
 * A comprehensive set of presets demonstrating cross-file analysis patterns:
 *
 * - default: General provide/inject and emit patterns
 * - reactivity: Reactivity loss patterns (destructuring, spreading)
 * - modules: Module-level state and CSRP risks
 * - reference-escape: Reference escape patterns
 * - provide-inject: Provide/inject type safety
 * - attrs: $attrs handling patterns
 *
 * Note: This file is separate from the Vue component to avoid
 * linting issues with embedded Vue code in template literals.
 */

import {
  mdiDiamond,
  mdiFlash,
  mdiAlert,
  mdiArrowTopRight,
  mdiFileTree,
  mdiArrowDown,
} from "@mdi/js";

export interface Preset {
  id: string;
  name: string;
  description: string;
  icon: string;
  files: Record<string, string>;
}

export const PRESETS: Preset[] = [
  {
    id: "default",
    name: "Overview",
    description: "General cross-file analysis patterns",
    icon: mdiDiamond,
    files: {
      "App.vue": `<script setup lang="ts">
import { provide, ref } from 'vue'
import ParentComponent from './ParentComponent.vue'

// Provide theme to all descendants
const theme = ref<'light' | 'dark'>('dark')
provide('theme', theme)
provide('user', { name: 'John', id: 1 })

function handleUpdate(value: number) {
  console.log('Updated:', value)
}
</script>

<template>
  <div id="app" class="app-container">
    <ParentComponent
      title="Dashboard"
      @update="handleUpdate"
    />
  </div>
</template>`,

      "ParentComponent.vue": `<script setup lang="ts">
import { inject, ref, onMounted } from 'vue'
import ChildComponent from './ChildComponent.vue'

const props = defineProps<{
  title: string
}>()

const emit = defineEmits<{
  update: [value: number]
  'unused-event': []
}>()

const theme = inject<Ref<'light' | 'dark'>>('theme')

// ISSUE: Destructuring inject loses reactivity!
const { name } = inject('user') as { name: string; id: number }

const width = ref(0)
onMounted(() => {
  width.value = window.innerWidth
})
</script>

<template>
  <div :class="['parent', theme]">
    <h2>{{ title }}</h2>
    <p>User: {{ name }}</p>
    <ChildComponent
      :theme="theme"
      custom-attr="value"
      @change="emit('update', $event)"
    />
  </div>
</template>`,

      "ChildComponent.vue": `<script setup lang="ts">
import { ref, toRefs } from 'vue'

const props = defineProps<{
  theme?: string
}>()

const { theme } = toRefs(props)

const emit = defineEmits<{
  change: [value: number]
}>()

const items = ref([
  { id: 1, name: 'Item 1' },
  { id: 2, name: 'Item 2' },
])

function handleClick(item: { id: number; name: string }) {
  emit('change', item.id)
}
</script>

<template>
  <!-- ISSUE: Multiple root elements without v-bind="$attrs" -->
  <div class="child-header">
    <span>Theme: {{ theme }}</span>
  </div>
  <ul class="child-list">
    <li v-for="item in items" :key="item.id" @click="handleClick(item)">
      {{ item.name }}
    </li>
  </ul>
</template>`,
    },
  },

  {
    id: "reactivity-loss",
    name: "Reactivity Loss",
    description: "Patterns that break Vue reactivity",
    icon: mdiFlash,
    files: {
      "App.vue": `<script setup lang="ts">
import { reactive, ref, provide } from 'vue'
import ChildComponent from './ChildComponent.vue'

// === Correct Usage ===
const state = reactive({
  count: 0,
  user: { name: 'Alice', age: 25 }
})

// === ANTI-PATTERNS: Reactivity Loss ===

// 1. Destructuring reactive object breaks reactivity
const { count, user } = state  // ❌ count is now a plain number

// 2. Spreading reactive object breaks reactivity
const copiedState = { ...state }  // ❌ No longer reactive

// 3. Reassigning reactive variable breaks reactivity
let dynamicState = reactive({ value: 1 })
dynamicState = reactive({ value: 2 })  // ❌ Original tracking lost

// 4. Extracting primitive from ref
const countRef = ref(10)
const primitiveValue = countRef.value  // ❌ Just a number, not reactive

provide('state', state)
</script>

<template>
  <div>
    <h1>Reactivity Loss Patterns</h1>
    <p>Count: {{ count }}</p>
    <p>User: {{ user.name }}</p>
    <ChildComponent />
  </div>
</template>`,

      "ChildComponent.vue": `<script setup lang="ts">
import { inject, computed, toRefs, toRef } from 'vue'

const state = inject('state') as { count: number; user: { name: string } }

// === ANTI-PATTERNS ===

// 1. Destructuring inject result (this will trigger a warning)
const { count } = state  // ❌ Loses reactivity

// 2. This one is intentionally suppressed with @vize forget
// @vize forget: intentionally reading one-time value
const userName = state.user.name  // This warning is suppressed

// === CORRECT PATTERNS ===

// Use toRef for single property
const countRef = toRef(state, 'count')

// Use toRefs for multiple properties
const { user } = toRefs(state as any)

// Use computed for derived values
const displayName = computed(() => state.user.name.toUpperCase())
</script>

<template>
  <div>
    <h2>Child Component</h2>
    <p>Broken count: {{ count }}</p>
    <p>Reactive count: {{ countRef }}</p>
    <p>Display name: {{ displayName }}</p>
  </div>
</template>`,

      "stores/user.ts": `import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export const useUserStore = defineStore('user', () => {
  const username = ref('john_doe')
  const email = ref('john@example.com')

  const displayName = computed(() => username.value.toUpperCase())

  function updateUser(name: string, mail: string) {
    username.value = name
    email.value = mail
  }

  return { username, email, displayName, updateUser }
})
`,

      "StoreExample.vue": `<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useUserStore } from './stores/user'

const userStore = useUserStore()

// ❌ WRONG: Destructuring Pinia store loses reactivity for state/getters
const { username, email } = userStore

// ✓ CORRECT: Use storeToRefs for reactive state/getters
// const { username, email } = storeToRefs(userStore)

// ✓ Actions can be destructured directly (they're just functions)
// const { updateUser } = userStore
</script>

<template>
  <div>
    <p>Username: {{ username }}</p>
    <p>Email: {{ email }}</p>
  </div>
</template>`,

      "SpreadPattern.vue": `<script setup lang="ts">
import { reactive, ref, toRaw } from 'vue'

interface User {
  id: number
  name: string
  settings: { theme: string }
}

const user = reactive<User>({
  id: 1,
  name: 'Bob',
  settings: { theme: 'dark' }
})

// === SPREAD ANTI-PATTERNS ===

// ❌ Spreading reactive object
const userCopy = { ...user }

// ❌ Spreading in function call
function logUser(u: User) {
  console.log(u)
}
logUser({ ...user })

// ❌ Array spread on reactive array
const items = reactive([1, 2, 3])
const itemsCopy = [...items]

// === CORRECT PATTERNS ===

// ✓ Use toRaw if you need plain object
const rawUser = toRaw(user)

// ✓ Clone with structuredClone for deep copy
const deepCopy = structuredClone(toRaw(user))

// ✓ Pass reactive object directly
logUser(user)
</script>

<template>
  <div>
    <p>Original: {{ user.name }}</p>
    <p>Copy (not reactive): {{ userCopy.name }}</p>
  </div>
</template>`,
    },
  },

  {
    id: "setup-context",
    name: "Setup Context",
    description: "Vue APIs called outside setup (CSRP/Memory Leak)",
    icon: mdiAlert,
    files: {
      "App.vue": `<script setup lang="ts">
import ComponentWithLeaks from './ComponentWithLeaks.vue'
import SafeComponent from './SafeComponent.vue'
</script>

<template>
  <div>
    <h1>Setup Context Violations</h1>
    <p>CSRP = Cross-request State Pollution (SSR)</p>
    <p>Memory Leaks from watchers created outside setup</p>
    <ComponentWithLeaks />
    <SafeComponent />
  </div>
</template>`,

      "ComponentWithLeaks.vue": `<script setup lang="ts">
import { ref, watch, onMounted, computed, provide, inject } from 'vue'
import { createGlobalState } from './utils/state'
</script>

<script lang="ts">
// ⚠️ WARNING: Module-level Vue APIs cause issues!

import { ref, reactive, watch, computed, provide } from 'vue'

// ❌ CSRP Risk: Module-level reactive state is shared across requests in SSR
const globalCounter = ref(0)

// ❌ CSRP Risk: Module-level reactive object
const sharedState = reactive({
  users: [],
  settings: {}
})

// ❌ Memory Leak: Watch created outside setup is never cleaned up
watch(globalCounter, (val) => {
  console.log('Counter changed:', val)
})

// ❌ Memory Leak: Computed outside setup
const doubledCounter = computed(() => globalCounter.value * 2)

// ❌ Invalid: Provide outside setup
// provide('counter', globalCounter)  // This would throw!

export default {
  name: 'ComponentWithLeaks'
}
</script>

<template>
  <div class="warning-box">
    <h2>Component with Issues</h2>
    <p>Global counter: {{ globalCounter }}</p>
    <p>This component has CSRP risks and memory leaks!</p>
  </div>
</template>`,

      "SafeComponent.vue": `<script setup lang="ts">
import { ref, reactive, watch, computed, provide, onUnmounted } from 'vue'

// ✓ CORRECT: All Vue APIs inside setup context

// ✓ Component-scoped reactive state
const counter = ref(0)
const state = reactive({
  items: [] as string[]
})

// ✓ Watch inside setup - auto-cleaned up
watch(counter, (val) => {
  console.log('Counter changed:', val)
})

// ✓ Computed inside setup
const doubled = computed(() => counter.value * 2)

// ✓ Provide inside setup
provide('counter', counter)

// ✓ If you need manual cleanup
const customEffect = () => {
  // some side effect
}
onUnmounted(() => {
  // cleanup
})

function increment() {
  counter.value++
}
</script>

<template>
  <div class="safe-box">
    <h2>Safe Component</h2>
    <p>Counter: {{ counter }} (doubled: {{ doubled }})</p>
    <button @click="increment">Increment</button>
    <p>All Vue APIs properly scoped to setup context</p>
  </div>
</template>`,

      "utils/state.ts": `import { ref, reactive, computed, watch } from 'vue'

// ❌ DANGEROUS: Factory function that creates reactive state at module level
// Each import shares the same state!

// This file demonstrates why you should NOT do this:

const moduleState = reactive({
  value: 0
})

// ❌ Module-level watch - memory leak!
watch(() => moduleState.value, (v) => console.log(v))

// ✓ CORRECT: Factory function that creates fresh state per call
export function createGlobalState() {
  const state = reactive({
    value: 0
  })

  // This watch will only be created when the function is called
  // inside a setup context, ensuring proper cleanup
  return {
    state,
    increment: () => state.value++
  }
}

// ✓ CORRECT: Use VueUse's createGlobalState for shared state
// import { createGlobalState } from '@vueuse/core'
// export const useGlobalState = createGlobalState(() => reactive({ count: 0 }))
`,
    },
  },

  {
    id: "reference-escape",
    name: "Reference Escape",
    description: "Reactive references escaping scope (Rust-like tracking)",
    icon: mdiArrowTopRight,
    files: {
      "App.vue": `<script setup lang="ts">
import { reactive, ref, provide } from 'vue'
import ChildComponent from './ChildComponent.vue'
import { useExternalStore } from './stores/external'

// === REFERENCE ESCAPE PATTERNS ===

const state = reactive({
  user: { name: 'Alice', permissions: ['read'] },
  items: [] as string[]
})

// ❌ ESCAPE: Passing reactive object to external function
// The external function may store a reference
useExternalStore().registerState(state)

// ❌ ESCAPE: Assigning to window/global
;(window as any).appState = state

// ❌ ESCAPE: Returning from setup to be used elsewhere
// (This is often intentional via provide, but needs awareness)
provide('state', state)

function addItem(item: string) {
  state.items.push(item)
}
</script>

<template>
  <div>
    <h1>Reference Escape Tracking</h1>
    <p>User: {{ state.user.name }}</p>
    <ChildComponent :state="state" @add="addItem" />
  </div>
</template>`,

      "ChildComponent.vue": `<script setup lang="ts">
import { inject, watch, onUnmounted } from 'vue'

const props = defineProps<{
  state: { user: { name: string }; items: string[] }
}>()

const emit = defineEmits<{
  add: [item: string]
}>()

// ❌ ESCAPE: Storing prop reference in external location
let cachedState: typeof props.state | null = null
function cacheState() {
  cachedState = props.state  // Reference escapes!
}

// ❌ ESCAPE: setTimeout/setInterval with reactive reference
setTimeout(() => {
  // This closure captures props.state
  console.log(props.state.user.name)
}, 1000)

// ❌ ESCAPE: Event listener with reactive reference
function setupListener() {
  document.addEventListener('click', () => {
    // Reference escapes to global event listener!
    console.log(props.state.items.length)
  })
}

// ✓ CORRECT: Use local copy or computed if needed
import { computed, readonly } from 'vue'
const userName = computed(() => props.state.user.name)
const readonlyState = readonly(props.state)  // Prevent accidental mutations
</script>

<template>
  <div>
    <h2>Child Component</h2>
    <p>User: {{ userName }}</p>
    <button @click="emit('add', 'new item')">Add Item</button>
  </div>
</template>`,

      "stores/external.ts": `import { reactive } from 'vue'

interface State {
  user: { name: string; permissions: string[] }
  items: string[]
}

// This simulates an external store that holds references
class ExternalStore {
  // Using object type to store states by key
  private states: { [key: string]: State } = {}

  // ❌ This stores a reference to reactive object
  registerState(state: State) {
    // The reactive object is now stored externally
    // Mutations here affect the original!
    this.states['main'] = state

    // ❌ DANGER: External code can mutate your reactive state
    setTimeout(() => {
      state.user.name = 'Modified externally!'
    }, 5000)
  }

  getState(key: string) {
    return this.states[key]
  }
}

// Singleton - state persists across component lifecycle
const store = new ExternalStore()

export function useExternalStore() {
  return store
}
`,

      "SafePattern.vue": `<script setup lang="ts">
import { reactive, toRaw, readonly, shallowRef, markRaw, onUnmounted } from 'vue'

// === SAFE PATTERNS FOR REFERENCE MANAGEMENT ===

const state = reactive({
  data: { value: 1 }
})

// ✓ SAFE: Pass raw copy to external APIs
function sendToAnalytics() {
  const raw = toRaw(state)
  const copy = structuredClone(raw)
  // analytics.track(copy)  // Safe - no reactive reference
}

// ✓ SAFE: Use readonly for external exposure
const publicState = readonly(state)

// ✓ SAFE: Use markRaw for data that shouldn't be reactive
const heavyObject = markRaw({
  largeArray: new Array(10000).fill(0),
  canvas: null as HTMLCanvasElement | null
})

// ✓ SAFE: Proper cleanup for external references
let cleanupFn: (() => void) | null = null

function setupExternalListener() {
  const handler = () => {
    // Use state here
  }
  document.addEventListener('scroll', handler)
  cleanupFn = () => document.removeEventListener('scroll', handler)
}

onUnmounted(() => {
  cleanupFn?.()
})
</script>

<template>
  <div>
    <h2>Safe Reference Patterns</h2>
    <p>Value: {{ state.data.value }}</p>
  </div>
</template>`,
    },
  },

  {
    id: "provide-inject",
    name: "Provide/Inject Tree",
    description: "Complex dependency injection patterns",
    icon: mdiFileTree,
    files: {
      "App.vue": `<script setup lang="ts">
import { provide, ref, reactive, readonly } from 'vue'
import type { InjectionKey } from 'vue'
import ThemeProvider from './ThemeProvider.vue'

// === TYPED INJECTION KEYS ===
export const UserKey: InjectionKey<{ name: string; role: string }> = Symbol('user')
export const ConfigKey: InjectionKey<{ apiUrl: string }> = Symbol('config')

// ✓ Provide typed values
const user = reactive({ name: 'Admin', role: 'admin' })
provide(UserKey, readonly(user))

// ✓ Provide config
provide(ConfigKey, { apiUrl: 'https://api.example.com' })

// ❌ Untyped provide - consumers may use wrong type
provide('legacyData', { foo: 'bar' })

// ❌ Provide without consumer
provide('unusedKey', 'this is never injected')
</script>

<template>
  <div>
    <h1>Provide/Inject Patterns</h1>
    <ThemeProvider>
      <slot />
    </ThemeProvider>
  </div>
</template>`,

      "ThemeProvider.vue": `<script setup lang="ts">
import { provide, ref, computed, inject } from 'vue'
import type { InjectionKey, Ref, ComputedRef } from 'vue'
import SettingsPanel from './SettingsPanel.vue'

// === THEME INJECTION KEY ===
export interface ThemeContext {
  theme: Ref<'light' | 'dark'>
  toggleTheme: () => void
  isDark: ComputedRef<boolean>
}
export const ThemeKey: InjectionKey<ThemeContext> = Symbol('theme')

const theme = ref<'light' | 'dark'>('dark')
const toggleTheme = () => {
  theme.value = theme.value === 'light' ? 'dark' : 'light'
}
const isDark = computed(() => theme.value === 'dark')

provide(ThemeKey, {
  theme,
  toggleTheme,
  isDark,
})

// Also provide CSS variables approach
provide('cssVars', computed(() => ({
  '--bg-color': isDark.value ? '#1a1a1a' : '#ffffff',
  '--text-color': isDark.value ? '#ffffff' : '#1a1a1a',
})))
</script>

<template>
  <div :class="['theme-provider', theme]">
    <SettingsPanel />
    <slot />
  </div>
</template>`,

      "SettingsPanel.vue": `<script setup lang="ts">
import { inject } from 'vue'
import { ThemeKey, type ThemeContext } from './ThemeProvider.vue'
import { UserKey, ConfigKey } from './App.vue'

// ✓ Typed inject with Symbol key
const theme = inject(ThemeKey)
if (!theme) {
  throw new Error('ThemeProvider not found')
}

// ✓ Inject user with type safety
const user = inject(UserKey)

// ❌ Inject with default - may hide missing provider
const config = inject(ConfigKey, { apiUrl: 'http://localhost:3000' })

// ❌ Untyped inject - no type safety
const legacyData = inject('legacyData') as { foo: string }

// ❌ Inject key that doesn't exist (without default)
// const missing = inject('nonExistentKey')  // Would be undefined!

// ❌ Destructuring inject loses reactivity!
const { foo } = inject('legacyData') as { foo: string }
</script>

<template>
  <div class="settings-panel">
    <h2>Settings</h2>
    <p>Theme: {{ theme.theme.value }}</p>
    <p>User: {{ user?.name ?? 'Unknown' }}</p>
    <p>API: {{ config.apiUrl }}</p>
    <button @click="theme.toggleTheme">Toggle Theme</button>
  </div>
</template>`,

      "DeepChild.vue": `<script setup lang="ts">
import { inject, computed } from 'vue'
import { ThemeKey } from './ThemeProvider.vue'
import { UserKey } from './App.vue'

// ✓ Inject works at any depth
const theme = inject(ThemeKey)
const user = inject(UserKey)

// ✓ Create computed from injected values
const greeting = computed(() => {
  if (!user) return 'Hello!'
  return \`Hello, \${user.name}! You are \${user.role}\`
})

const themeClass = computed(() => theme?.isDark.value ? 'dark-mode' : 'light-mode')
</script>

<template>
  <div :class="['deep-child', themeClass]">
    <h3>Deep Child Component</h3>
    <p>{{ greeting }}</p>
    <p v-if="theme">Current theme: {{ theme.theme.value }}</p>
  </div>
</template>`,
    },
  },

  {
    id: "fallthrough-attrs",
    name: "Fallthrough Attrs",
    description: "$attrs, useAttrs(), and inheritAttrs patterns",
    icon: mdiArrowDown,
    files: {
      "App.vue": `<script setup lang="ts">
import BaseButton from './BaseButton.vue'
import MultiRootComponent from './MultiRootComponent.vue'
import UseAttrsComponent from './UseAttrsComponent.vue'
</script>

<template>
  <div>
    <h1>Fallthrough Attributes</h1>

    <!-- Passing class, style, and event to child -->
    <BaseButton
      class="custom-class"
      style="color: red"
      data-testid="main-button"
      @click="console.log('clicked')"
    >
      Click me
    </BaseButton>

    <!-- Multi-root needs explicit $attrs binding -->
    <MultiRootComponent
      class="passed-class"
      aria-label="Multiple roots"
    />

    <!-- Component using useAttrs() -->
    <UseAttrsComponent
      class="attrs-class"
      custom-attr="value"
    />
  </div>
</template>`,

      "BaseButton.vue": `<script setup lang="ts">
// Single root element - $attrs automatically applied

defineProps<{
  variant?: 'primary' | 'secondary'
}>()
</script>

<template>
  <!-- ✓ $attrs (class, style, listeners) auto-applied to single root -->
  <button class="base-button">
    <slot />
  </button>
</template>`,

      "MultiRootComponent.vue": `<script setup lang="ts">
// ❌ Multiple root elements - $attrs not auto-applied!
// Need to explicitly bind $attrs to intended element
</script>

<template>
  <!-- ❌ Which element gets class="passed-class"? Neither! -->
  <header class="header">
    Header content
  </header>
  <main class="main">
    Main content
  </main>
  <footer class="footer">
    Footer content
  </footer>
</template>`,

      "MultiRootFixed.vue": `<script setup lang="ts">
// ✓ Multiple roots with explicit $attrs binding
</script>

<template>
  <header class="header">
    Header content
  </header>
  <!-- ✓ Explicitly bind $attrs to main element -->
  <main v-bind="$attrs" class="main">
    Main content
  </main>
  <footer class="footer">
    Footer content
  </footer>
</template>`,

      "UseAttrsComponent.vue": `<script setup lang="ts">
import { useAttrs, computed } from 'vue'

// ✓ useAttrs() for programmatic access
const attrs = useAttrs()

// Access specific attributes
const customAttr = computed(() => attrs['custom-attr'])

// ❌ useAttrs() called but attrs not bound in template!
// This means passed attributes are lost
</script>

<template>
  <div>
    <p>Custom attr value: {{ customAttr }}</p>
    <!-- ❌ attrs not bound - class="attrs-class" is lost! -->
  </div>
</template>`,

      "UseAttrsFixed.vue": `<script setup lang="ts">
import { useAttrs, computed } from 'vue'

const attrs = useAttrs()
const customAttr = computed(() => attrs['custom-attr'])

// ✓ Can filter/transform attrs
const filteredAttrs = computed(() => {
  const { class: _, ...rest } = attrs
  return rest
})
</script>

<template>
  <!-- ✓ Explicitly bind attrs -->
  <div v-bind="attrs">
    <p>Custom attr: {{ customAttr }}</p>
  </div>
</template>`,

      "InheritAttrsFalse.vue": `<script setup lang="ts">
// ❌ inheritAttrs: false but $attrs not used!
// Passed attributes are completely lost

defineOptions({
  inheritAttrs: false
})
</script>

<template>
  <div class="wrapper">
    <input type="text" />
    <!-- $attrs should be bound to input, not wrapper -->
  </div>
</template>`,

      "InheritAttrsFixed.vue": `<script setup lang="ts">
// ✓ inheritAttrs: false with explicit $attrs binding

defineOptions({
  inheritAttrs: false
})
</script>

<template>
  <div class="wrapper">
    <!-- ✓ Bind $attrs to the actual input -->
    <input v-bind="$attrs" type="text" />
  </div>
</template>`,
    },
  },
];
