<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import MonacoEditor from './MonacoEditor.vue';
import type { Diagnostic } from './MonacoEditor.vue';
import type { WasmModule, CroquisResult, CrossFileResult, CrossFileInput, CrossFileOptions as WasmCrossFileOptions } from '../wasm/index';
import { getWasm } from '../wasm/index';
import {
  mdiDiamond,
  mdiFlash,
  mdiAlert,
  mdiArrowTopRight,
  mdiFileTree,
  mdiArrowDown,
  mdiLanguageTypescript,
  mdiVuejs,
  mdiFile,
  mdiClose,
  mdiInformation,
  mdiCheck,
  mdiLightbulbOn,
} from '@mdi/js';

const props = defineProps<{
  compiler: WasmModule | null;
}>();

// === Presets ===
interface Preset {
  id: string;
  name: string;
  description: string;
  icon: string;
  files: Record<string, string>;
}

const PRESETS: Preset[] = [
  {
    id: 'default',
    name: 'Overview',
    description: 'General cross-file analysis patterns',
    icon: mdiDiamond,
    files: {
      'App.vue': `<script setup lang="ts">
import { provide, ref } from 'vue'
import ParentComponent from './ParentComponent.vue'

// Provide theme to all descendants
const theme = ref<'light' | 'dark'>('dark')
provide('theme', theme)
provide('user', { name: 'John', id: 1 })

function handleUpdate(value: number) {
  console.log('Updated:', value)
}
<\/script>

<template>
  <div id="app" class="app-container">
    <ParentComponent
      title="Dashboard"
      @update="handleUpdate"
    />
  </div>
</template>`,

      'ParentComponent.vue': `<script setup lang="ts">
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
<\/script>

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

      'ChildComponent.vue': `<script setup lang="ts">
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
<\/script>

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
    id: 'reactivity-loss',
    name: 'Reactivity Loss',
    description: 'Patterns that break Vue reactivity',
    icon: mdiFlash,
    files: {
      'App.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div>
    <h1>Reactivity Loss Patterns</h1>
    <p>Count: {{ count }}</p>
    <p>User: {{ user.name }}</p>
    <ChildComponent />
  </div>
</template>`,

      'ChildComponent.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div>
    <h2>Child Component</h2>
    <p>Broken count: {{ count }}</p>
    <p>Reactive count: {{ countRef }}</p>
    <p>Display name: {{ displayName }}</p>
  </div>
</template>`,

      'stores/user.ts': `import { defineStore } from 'pinia'
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

      'StoreExample.vue': `<script setup lang="ts">
import { storeToRefs } from 'pinia'
import { useUserStore } from './stores/user'

const userStore = useUserStore()

// ❌ WRONG: Destructuring Pinia store loses reactivity for state/getters
const { username, email } = userStore

// ✓ CORRECT: Use storeToRefs for reactive state/getters
// const { username, email } = storeToRefs(userStore)

// ✓ Actions can be destructured directly (they're just functions)
// const { updateUser } = userStore
<\/script>

<template>
  <div>
    <p>Username: {{ username }}</p>
    <p>Email: {{ email }}</p>
  </div>
</template>`,

      'SpreadPattern.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div>
    <p>Original: {{ user.name }}</p>
    <p>Copy (not reactive): {{ userCopy.name }}</p>
  </div>
</template>`,
    },
  },

  {
    id: 'setup-context',
    name: 'Setup Context',
    description: 'Vue APIs called outside setup (CSRP/Memory Leak)',
    icon: mdiAlert,
    files: {
      'App.vue': `<script setup lang="ts">
import ComponentWithLeaks from './ComponentWithLeaks.vue'
import SafeComponent from './SafeComponent.vue'
<\/script>

<template>
  <div>
    <h1>Setup Context Violations</h1>
    <p>CSRP = Cross-request State Pollution (SSR)</p>
    <p>Memory Leaks from watchers created outside setup</p>
    <ComponentWithLeaks />
    <SafeComponent />
  </div>
</template>`,

      'ComponentWithLeaks.vue': `<script setup lang="ts">
import { ref, watch, onMounted, computed, provide, inject } from 'vue'
import { createGlobalState } from './utils/state'
<\/script>

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
<\/script>

<template>
  <div class="warning-box">
    <h2>Component with Issues</h2>
    <p>Global counter: {{ globalCounter }}</p>
    <p>This component has CSRP risks and memory leaks!</p>
  </div>
</template>`,

      'SafeComponent.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div class="safe-box">
    <h2>Safe Component</h2>
    <p>Counter: {{ counter }} (doubled: {{ doubled }})</p>
    <button @click="increment">Increment</button>
    <p>All Vue APIs properly scoped to setup context</p>
  </div>
</template>`,

      'utils/state.ts': `import { ref, reactive, computed, watch } from 'vue'

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
    id: 'reference-escape',
    name: 'Reference Escape',
    description: 'Reactive references escaping scope (Rust-like tracking)',
    icon: mdiArrowTopRight,
    files: {
      'App.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div>
    <h1>Reference Escape Tracking</h1>
    <p>User: {{ state.user.name }}</p>
    <ChildComponent :state="state" @add="addItem" />
  </div>
</template>`,

      'ChildComponent.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div>
    <h2>Child Component</h2>
    <p>User: {{ userName }}</p>
    <button @click="emit('add', 'new item')">Add Item</button>
  </div>
</template>`,

      'stores/external.ts': `import { reactive } from 'vue'

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

      'SafePattern.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div>
    <h2>Safe Reference Patterns</h2>
    <p>Value: {{ state.data.value }}</p>
  </div>
</template>`,
    },
  },

  {
    id: 'provide-inject',
    name: 'Provide/Inject Tree',
    description: 'Complex dependency injection patterns',
    icon: mdiFileTree,
    files: {
      'App.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div>
    <h1>Provide/Inject Patterns</h1>
    <ThemeProvider>
      <slot />
    </ThemeProvider>
  </div>
</template>`,

      'ThemeProvider.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div :class="['theme-provider', theme]">
    <SettingsPanel />
    <slot />
  </div>
</template>`,

      'SettingsPanel.vue': `<script setup lang="ts">
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
<\/script>

<template>
  <div class="settings-panel">
    <h2>Settings</h2>
    <p>Theme: {{ theme.theme.value }}</p>
    <p>User: {{ user?.name ?? 'Unknown' }}</p>
    <p>API: {{ config.apiUrl }}</p>
    <button @click="theme.toggleTheme">Toggle Theme</button>
  </div>
</template>`,

      'DeepChild.vue': `<script setup lang="ts">
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
<\/script>

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
    id: 'fallthrough-attrs',
    name: 'Fallthrough Attrs',
    description: '$attrs, useAttrs(), and inheritAttrs patterns',
    icon: mdiArrowDown,
    files: {
      'App.vue': `<script setup lang="ts">
import BaseButton from './BaseButton.vue'
import MultiRootComponent from './MultiRootComponent.vue'
import UseAttrsComponent from './UseAttrsComponent.vue'
<\/script>

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

      'BaseButton.vue': `<script setup lang="ts">
// Single root element - $attrs automatically applied

defineProps<{
  variant?: 'primary' | 'secondary'
}>()
<\/script>

<template>
  <!-- ✓ $attrs (class, style, listeners) auto-applied to single root -->
  <button class="base-button">
    <slot />
  </button>
</template>`,

      'MultiRootComponent.vue': `<script setup lang="ts">
// ❌ Multiple root elements - $attrs not auto-applied!
// Need to explicitly bind $attrs to intended element
<\/script>

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

      'MultiRootFixed.vue': `<script setup lang="ts">
// ✓ Multiple roots with explicit $attrs binding
<\/script>

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

      'UseAttrsComponent.vue': `<script setup lang="ts">
import { useAttrs, computed } from 'vue'

// ✓ useAttrs() for programmatic access
const attrs = useAttrs()

// Access specific attributes
const customAttr = computed(() => attrs['custom-attr'])

// ❌ useAttrs() called but attrs not bound in template!
// This means passed attributes are lost
<\/script>

<template>
  <div>
    <p>Custom attr value: {{ customAttr }}</p>
    <!-- ❌ attrs not bound - class="attrs-class" is lost! -->
  </div>
</template>`,

      'UseAttrsFixed.vue': `<script setup lang="ts">
import { useAttrs, computed } from 'vue'

const attrs = useAttrs()
const customAttr = computed(() => attrs['custom-attr'])

// ✓ Can filter/transform attrs
const filteredAttrs = computed(() => {
  const { class: _, ...rest } = attrs
  return rest
})
<\/script>

<template>
  <!-- ✓ Explicitly bind attrs -->
  <div v-bind="attrs">
    <p>Custom attr: {{ customAttr }}</p>
  </div>
</template>`,

      'InheritAttrsFalse.vue': `<script setup lang="ts">
// ❌ inheritAttrs: false but $attrs not used!
// Passed attributes are completely lost

defineOptions({
  inheritAttrs: false
})
<\/script>

<template>
  <div class="wrapper">
    <input type="text" />
    <!-- $attrs should be bound to input, not wrapper -->
  </div>
</template>`,

      'InheritAttrsFixed.vue': `<script setup lang="ts">
// ✓ inheritAttrs: false with explicit $attrs binding

defineOptions({
  inheritAttrs: false
})
<\/script>

<template>
  <div class="wrapper">
    <!-- ✓ Bind $attrs to the actual input -->
    <input v-bind="$attrs" type="text" />
  </div>
</template>`,
    },
  },
];

// === State ===
const currentPreset = ref<string>('default');
const currentPresetData = computed(() => PRESETS.find(p => p.id === currentPreset.value) || PRESETS[0]);
const files = ref<Record<string, string>>({ ...currentPresetData.value.files });
const activeFile = ref<string>(Object.keys(currentPresetData.value.files)[0]);

// Monaco editor ref (for direct setValue calls - vite-plugin-vize workaround)
const monacoEditorRef = ref<InstanceType<typeof MonacoEditor> | null>(null);

// File names array for v-for (workaround for vite-plugin-vize object iteration issue)
// Using ref + watch instead of computed to ensure reactivity works correctly
const fileNames = ref<string[]>(Object.keys(files.value));
watch(files, (newFiles) => {
  fileNames.value = Object.keys(newFiles);
}, { deep: true });
const croquisResults = ref<Record<string, CroquisResult | null>>({});
const crossFileIssues = ref<CrossFileIssue[]>([]);
const analysisTime = ref<number>(0);
const isAnalyzing = ref(false);
const selectedIssue = ref<CrossFileIssue | null>(null);

// Options
const options = ref({
  provideInject: true,
  componentEmits: true,
  fallthroughAttrs: true,
  reactivityTracking: true,
  uniqueIds: true,
  serverClientBoundary: true,
});

// === Resizable Panes ===
const sidebarWidth = ref(220);
const diagnosticsWidth = ref(320);
const isResizingSidebar = ref(false);
const isResizingDiagnostics = ref(false);
const containerRef = ref<HTMLElement | null>(null);

function startSidebarResize(e: MouseEvent) {
  isResizingSidebar.value = true;
  e.preventDefault();
  document.addEventListener('mousemove', onSidebarResize);
  document.addEventListener('mouseup', stopResize);
}

function startDiagnosticsResize(e: MouseEvent) {
  isResizingDiagnostics.value = true;
  e.preventDefault();
  document.addEventListener('mousemove', onDiagnosticsResize);
  document.addEventListener('mouseup', stopResize);
}

function onSidebarResize(e: MouseEvent) {
  if (!isResizingSidebar.value || !containerRef.value) return;
  const containerRect = containerRef.value.getBoundingClientRect();
  const newWidth = Math.max(150, Math.min(400, e.clientX - containerRect.left));
  sidebarWidth.value = newWidth;
}

function onDiagnosticsResize(e: MouseEvent) {
  if (!isResizingDiagnostics.value || !containerRef.value) return;
  const containerRect = containerRef.value.getBoundingClientRect();
  const newWidth = Math.max(200, Math.min(500, containerRect.right - e.clientX));
  diagnosticsWidth.value = newWidth;
}

function stopResize() {
  isResizingSidebar.value = false;
  isResizingDiagnostics.value = false;
  document.removeEventListener('mousemove', onSidebarResize);
  document.removeEventListener('mousemove', onDiagnosticsResize);
  document.removeEventListener('mouseup', stopResize);
}

const gridStyle = computed(() => ({
  gridTemplateColumns: `${sidebarWidth.value}px 4px 1fr 4px ${diagnosticsWidth.value}px`,
}));

// === Types ===
interface CrossFileIssue {
  id: string;
  type: string;
  code: string;
  severity: 'error' | 'warning' | 'info';
  message: string;
  file: string;
  line: number;
  column: number;
  endLine?: number;
  endColumn?: number;
  relatedLocations?: Array<{ file: string; line: number; column: number; message: string }>;
  suggestion?: string;
}

// === Source State (workaround for vite-plugin-vize reactivity issue) ===
const currentSource = ref(files.value[activeFile.value] || '');

// Sync currentSource when activeFile changes
watch(activeFile, (newFile) => {
  currentSource.value = files.value[newFile] || '';
}, { immediate: false });

// Sync files when currentSource changes (for editor input)
watch(currentSource, (newSource) => {
  files.value[activeFile.value] = newSource;
}, { immediate: false });

const currentDiagnostics = computed((): Diagnostic[] => {
  return crossFileIssues.value
    .filter(issue => issue.file === activeFile.value)
    .map(issue => ({
      message: `[${issue.code}] ${issue.message}${issue.suggestion ? `\n\nTip: ${issue.suggestion}` : ''}`,
      startLine: issue.line,
      startColumn: issue.column,
      endLine: issue.endLine,
      endColumn: issue.endColumn,
      severity: issue.severity,
    }));
});

const issuesByFile = computed(() => {
  const grouped: Record<string, CrossFileIssue[]> = {};
  for (const issue of crossFileIssues.value) {
    if (!grouped[issue.file]) grouped[issue.file] = [];
    grouped[issue.file].push(issue);
  }
  return grouped;
});

const issuesByType = computed(() => {
  const grouped: Record<string, CrossFileIssue[]> = {};
  for (const issue of crossFileIssues.value) {
    if (!grouped[issue.type]) grouped[issue.type] = [];
    grouped[issue.type].push(issue);
  }
  return grouped;
});

const stats = computed(() => ({
  files: Object.keys(files.value).length,
  totalIssues: crossFileIssues.value.length,
  errors: crossFileIssues.value.filter(i => i.severity === 'error').length,
  warnings: crossFileIssues.value.filter(i => i.severity === 'warning').length,
  infos: crossFileIssues.value.filter(i => i.severity === 'info').length,
}));

const editorLanguage = computed(() => {
  const ext = activeFile.value.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'ts':
      return 'typescript';
    case 'js':
      return 'javascript';
    case 'css':
      return 'css';
    case 'scss':
      return 'scss';
    case 'json':
      return 'json';
    case 'vue':
    default:
      return 'vue';
  }
});

const dependencyGraph = computed(() => {
  // Build simple dependency graph from imports
  const graph: Record<string, string[]> = {};
  for (const [filename, source] of Object.entries(files.value)) {
    const imports: string[] = [];
    const importRegex = /import\s+[\w{}\s,*]+\s+from\s+['"]\.\/([^'"]+)['"]/g;
    let match;
    while ((match = importRegex.exec(source)) !== null) {
      let importFile = match[1];
      if (!importFile.endsWith('.vue')) importFile += '.vue';
      if (files.value[importFile]) {
        imports.push(importFile);
      }
    }
    graph[filename] = imports;
  }
  return graph;
});

// === Analysis Functions ===
let issueIdCounter = 0;

function createIssue(
  type: string,
  code: string,
  severity: 'error' | 'warning' | 'info',
  message: string,
  file: string,
  line: number,
  column: number,
  options?: {
    endLine?: number;
    endColumn?: number;
    suggestion?: string;
    relatedLocations?: Array<{ file: string; line: number; column: number; message: string }>;
  }
): CrossFileIssue {
  return {
    id: `issue-${++issueIdCounter}`,
    type,
    code,
    severity,
    message,
    file,
    line,
    column,
    ...options,
  };
}

// Strip comments from source code to prevent regex matching comment content
function stripComments(source: string): string {
  // Remove single-line comments (// ...)
  // Remove multi-line comments (/* ... */)
  // Preserve string literals
  let result = '';
  let i = 0;
  while (i < source.length) {
    // Check for string literals
    if (source[i] === '"' || source[i] === "'" || source[i] === '`') {
      const quote = source[i];
      result += source[i++];
      while (i < source.length && source[i] !== quote) {
        if (source[i] === '\\' && i + 1 < source.length) {
          result += source[i++];
        }
        result += source[i++];
      }
      if (i < source.length) result += source[i++];
    }
    // Check for single-line comment
    else if (source[i] === '/' && source[i + 1] === '/') {
      // Replace with spaces to preserve offsets
      while (i < source.length && source[i] !== '\n') {
        result += ' ';
        i++;
      }
    }
    // Check for multi-line comment
    else if (source[i] === '/' && source[i + 1] === '*') {
      result += '  '; // Replace /* with spaces
      i += 2;
      while (i < source.length && !(source[i] === '*' && source[i + 1] === '/')) {
        result += source[i] === '\n' ? '\n' : ' ';
        i++;
      }
      if (i < source.length) {
        result += '  '; // Replace */ with spaces
        i += 2;
      }
    }
    else {
      result += source[i++];
    }
  }
  return result;
}

// Parse @vize forget suppression directives from source code
// Returns a set of line numbers that are suppressed (1-based)
function parseSuppressions(source: string): Set<number> {
  const suppressedLines = new Set<number>();
  const lines = source.split('\n');
  let pendingSuppression = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmedLine = line.trim();
    const lineNumber = i + 1; // 1-based

    // Check for @vize forget directive (supports both // and /* */ comments)
    // Patterns: // @vize forget: reason  OR  /* @vize forget: reason */
    const singleLineMatch = trimmedLine.match(/^\/\/\s*@vize\s+forget\s*:\s*(.+)/);
    const blockMatch = trimmedLine.match(/^\/\*\s*@vize\s+forget\s*:\s*(.+?)\s*\*\//);

    if ((singleLineMatch && singleLineMatch[1].trim()) || (blockMatch && blockMatch[1].trim())) {
      // Valid suppression - will apply to next non-comment, non-empty line
      pendingSuppression = true;
    } else if (pendingSuppression && trimmedLine && !trimmedLine.startsWith('//') && !trimmedLine.startsWith('/*')) {
      // This is a code line - apply pending suppression
      suppressedLines.add(lineNumber);
      pendingSuppression = false;
    }
  }

  return suppressedLines;
}

// Build suppression map for all files
function buildSuppressionMap(): Map<string, Set<number>> {
  const map = new Map<string, Set<number>>();
  for (const [filename, source] of Object.entries(files.value)) {
    map.set(filename, parseSuppressions(source));
  }
  return map;
}

// Filter issues based on suppression directives
function filterSuppressedIssues(issues: CrossFileIssue[], suppressionMap: Map<string, Set<number>>): CrossFileIssue[] {
  return issues.filter(issue => {
    const suppressedLines = suppressionMap.get(issue.file);
    if (!suppressedLines) return true;
    return !suppressedLines.has(issue.line);
  });
}

// Convert character offset to line/column (1-based for Monaco)
function offsetToLineColumn(source: string, offset: number): { line: number; column: number } {
  const beforeOffset = source.substring(0, offset);
  const lines = beforeOffset.split('\n');
  return {
    line: lines.length,
    column: lines[lines.length - 1].length + 1,
  };
}

// Find line/column for a pattern (uses first match)
function findLineAndColumn(source: string, pattern: RegExp | string): { line: number; column: number; endLine?: number; endColumn?: number } | null {
  const regex = typeof pattern === 'string' ? new RegExp(pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')) : pattern;
  const match = source.match(regex);
  if (!match || match.index === undefined) return null;

  const start = offsetToLineColumn(source, match.index);
  const end = offsetToLineColumn(source, match.index + match[0].length);

  return {
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
  };
}

// Find line/column at a specific offset (for regex exec results)
function findLineAndColumnAtOffset(source: string, offset: number, length: number): { line: number; column: number; endLine: number; endColumn: number } {
  const start = offsetToLineColumn(source, offset);
  const end = offsetToLineColumn(source, offset + length);
  return {
    line: start.line,
    column: start.column,
    endLine: end.line,
    endColumn: end.column,
  };
}

async function analyzeAll() {
  // Use getWasm() directly instead of props (workaround for vite-plugin-vize reactivity issue)
  const compiler = getWasm();
  if (!compiler) return;

  isAnalyzing.value = true;
  const startTime = performance.now();
  issueIdCounter = 0;

  // First pass: analyze each file with single-file analyzer
  const results: Record<string, CroquisResult | null> = {};
  for (const [filename, source] of Object.entries(files.value)) {
    try {
      results[filename] = compiler.analyzeSfc(source, { filename });
    } catch {
      results[filename] = null;
    }
  }
  croquisResults.value = results;

  // Prepare files for cross-file analysis
  const crossFileInputs: CrossFileInput[] = Object.entries(files.value).map(([path, source]) => ({
    path,
    source,
  }));

  // Build options for cross-file analysis
  const wasmOptions: WasmCrossFileOptions = {
    all: true, // Enable all analyzers for comprehensive analysis
    provideInject: options.value.provideInject,
    componentEmits: options.value.componentEmits,
    fallthroughAttrs: options.value.fallthroughAttrs,
    reactivityTracking: options.value.reactivityTracking,
    uniqueIds: options.value.uniqueIds,
    serverClientBoundary: options.value.serverClientBoundary,
  };

  let issues: CrossFileIssue[] = [];

  // Try WASM cross-file analysis first
  try {
    if (compiler.analyzeCrossFile) {
      const crossFileResult: CrossFileResult = compiler.analyzeCrossFile(crossFileInputs, wasmOptions);

      // Convert WASM diagnostics to CrossFileIssue format
      for (const diag of crossFileResult.diagnostics) {
        const source = files.value[diag.file] || '';
        const loc = offsetToLineColumn(source, diag.offset);
        const endLoc = offsetToLineColumn(source, diag.endOffset);

        issues.push({
          id: `issue-${++issueIdCounter}`,
          type: diag.type,
          code: diag.code,
          severity: diag.severity === 'hint' ? 'info' : diag.severity,
          message: diag.message,
          file: diag.file,
          line: loc.line,
          column: loc.column,
          endLine: endLoc.line,
          endColumn: endLoc.column,
          relatedLocations: diag.relatedLocations?.map(rel => {
            const relSource = files.value[rel.file] || '';
            const relLoc = offsetToLineColumn(relSource, rel.offset);
            return {
              file: rel.file,
              line: relLoc.line,
              column: relLoc.column,
              message: rel.message,
            };
          }),
          suggestion: diag.suggestion,
        });
      }

      analysisTime.value = crossFileResult.stats.analysisTimeMs;
    } else {
      // Fallback to TypeScript-based analysis if WASM not available
      issues = fallbackAnalysis();
      analysisTime.value = performance.now() - startTime;
    }
  } catch (e) {
    console.warn('WASM cross-file analysis failed, using fallback:', e);
    issues = fallbackAnalysis();
    analysisTime.value = performance.now() - startTime;
  }

  // Apply @vize forget suppression directives
  const suppressionMap = buildSuppressionMap();
  const filteredIssues = filterSuppressedIssues(issues, suppressionMap);

  crossFileIssues.value = filteredIssues;
  isAnalyzing.value = false;
}

// Fallback TypeScript-based analysis (for when WASM is not available)
function fallbackAnalysis(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  if (options.value.provideInject) {
    issues.push(...analyzeProvideInject());
  }
  if (options.value.componentEmits) {
    issues.push(...analyzeComponentEmits());
  }
  if (options.value.fallthroughAttrs) {
    issues.push(...analyzeFallthroughAttrs());
  }
  if (options.value.reactivityTracking) {
    issues.push(...analyzeReactivity());
  }
  if (options.value.uniqueIds) {
    issues.push(...analyzeUniqueIds());
  }
  if (options.value.serverClientBoundary) {
    issues.push(...analyzeSSRBoundary());
  }

  return issues;
}

function analyzeProvideInject(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const provides: Map<string, { file: string; line: number; column: number; endLine: number; endColumn: number; isSymbol: boolean }> = new Map();
  const injects: Array<{ key: string; file: string; line: number; column: number; endLine: number; endColumn: number; hasDefault: boolean; isSymbol: boolean; pattern?: string; destructuredProps?: string[] }> = [];

  // Use Rust analysis results instead of regex
  for (const [filename, source] of Object.entries(files.value)) {
    const result = croquisResults.value[filename];
    if (!result?.croquis) continue;

    // Collect provides from Rust analysis
    for (const p of result.croquis.provides || []) {
      const keyValue = p.key.type === 'symbol' ? `Symbol:${p.key.value}` : p.key.value;
      const loc = findLineAndColumnAtOffset(source, p.start, p.end - p.start);
      provides.set(keyValue, { file: filename, isSymbol: p.key.type === 'symbol', ...loc });
    }

    // Collect injects from Rust analysis
    for (const i of result.croquis.injects || []) {
      const keyValue = i.key.type === 'symbol' ? `Symbol:${i.key.value}` : i.key.value;
      const loc = findLineAndColumnAtOffset(source, i.start, i.end - i.start);
      injects.push({
        key: keyValue,
        file: filename,
        hasDefault: !!i.defaultValue,
        isSymbol: i.key.type === 'symbol',
        pattern: i.pattern,
        destructuredProps: i.destructuredProps,
        ...loc,
      });
    }
  }

  // Check for destructured injects (reactivity loss)
  for (const inject of injects) {
    if (inject.pattern === 'objectDestructure' || inject.pattern === 'arrayDestructure') {
      const displayKey = inject.isSymbol ? inject.key.replace('Symbol:', '') : `'${inject.key}'`;
      const propsStr = inject.destructuredProps?.join(', ') || '';
      issues.push(createIssue(
        'provide-inject',
        'cross-file/destructured-inject',
        'error',
        `Destructuring inject(${displayKey}) into { ${propsStr} } breaks reactivity`,
        inject.file,
        inject.line,
        inject.column,
        {
          endLine: inject.endLine,
          endColumn: inject.endColumn,
          suggestion: `Store inject result first, then access properties: const data = inject(${displayKey})`,
        }
      ));
    }
  }

  // Check for unmatched injects
  for (const inject of injects) {
    if (!provides.has(inject.key)) {
      const severity = inject.hasDefault ? 'info' : 'warning';
      const displayKey = inject.isSymbol ? inject.key.replace('Symbol:', '') : `'${inject.key}'`;
      const provideExample = inject.isSymbol
        ? `Add provide(${inject.key.replace('Symbol:', '')}, value) in a parent component`
        : `Add provide('${inject.key}', value) in a parent component`;
      issues.push(createIssue(
        'provide-inject',
        'cross-file/unmatched-inject',
        severity,
        `inject(${displayKey}) has no matching provide() in any ancestor component`,
        inject.file,
        inject.line,
        inject.column,
        {
          endLine: inject.endLine,
          endColumn: inject.endColumn,
          suggestion: inject.hasDefault
            ? 'Using default value since no provider found'
            : provideExample,
        }
      ));
    }
  }

  // Check for unused provides
  for (const [key, loc] of provides.entries()) {
    const hasConsumer = injects.some(i => i.key === key);
    if (!hasConsumer) {
      const displayKey = key.startsWith('Symbol:') ? key.replace('Symbol:', '') : `'${key}'`;
      issues.push(createIssue(
        'provide-inject',
        'cross-file/unused-provide',
        'info',
        `provide(${displayKey}) is not consumed by any descendant component`,
        loc.file,
        loc.line,
        loc.column,
        {
          endLine: loc.endLine,
          endColumn: loc.endColumn,
          suggestion: 'Remove if not needed, or add inject() in a child component',
        }
      ));
    }
  }

  return issues;
}

function analyzeComponentEmits(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  for (const [filename, source] of Object.entries(files.value)) {
    const result = croquisResults.value[filename];
    const codeOnly = stripComments(source);

    // Use Rust analysis for declared emits
    const declaredEmits: Array<{ name: string; loc: { line: number; column: number; endLine: number; endColumn: number } }> = [];

    if (result?.croquis?.emits) {
      // Get emit declarations from Rust analysis
      for (const emit of result.croquis.emits) {
        // Find location in source using defineEmits macro info
        const defineEmitsMacro = result.croquis.macros?.find(m => m.name === 'defineEmits');
        if (defineEmitsMacro) {
          const loc = findLineAndColumnAtOffset(source, defineEmitsMacro.start, defineEmitsMacro.end - defineEmitsMacro.start);
          declaredEmits.push({ name: emit.name, loc });
        }
      }
    } else {
      // Fallback to regex if Rust analysis not available
      const emitDeclRegex = /defineEmits\s*<\s*\{([^}]+)\}\s*>/s;
      const emitDeclMatch = emitDeclRegex.exec(source);
      if (emitDeclMatch && emitDeclMatch.index !== undefined) {
        const emitContent = emitDeclMatch[1];
        const emitContentOffset = emitDeclMatch.index + emitDeclMatch[0].indexOf(emitContent);

        // Style 1: Shorthand syntax
        const shorthandRegex = /(\w+)\s*:\s*\[/g;
        let match;
        while ((match = shorthandRegex.exec(emitContent)) !== null) {
          const absoluteOffset = emitContentOffset + match.index;
          const loc = findLineAndColumnAtOffset(source, absoluteOffset, match[1].length);
          declaredEmits.push({ name: match[1], loc });
        }

        // Style 2: Callback syntax
        const callbackRegex = /\(\s*e:\s*['"]([^'"]+)['"]/g;
        while ((match = callbackRegex.exec(emitContent)) !== null) {
          const absoluteOffset = emitContentOffset + match.index;
          const loc = findLineAndColumnAtOffset(source, absoluteOffset, match[0].length);
          declaredEmits.push({ name: match[1], loc });
        }
      }
    }

    if (declaredEmits.length === 0) continue;

    // Check if each declared emit is called (using regex on code without comments)
    for (const emit of declaredEmits) {
      const emitCallRegex = new RegExp(`emit\\s*\\(\\s*['"]${emit.name}['"]`, 'g');
      if (!emitCallRegex.test(codeOnly)) {
        issues.push(createIssue(
          'component-emit',
          'cross-file/unused-emit',
          'warning',
          `Event '${emit.name}' is declared in defineEmits but never emitted`,
          filename,
          emit.loc.line,
          emit.loc.column,
          {
            endLine: emit.loc.endLine,
            endColumn: emit.loc.endColumn,
            suggestion: `Remove '${emit.name}' from defineEmits if not needed`,
          }
        ));
      }
    }

    // Check for undeclared emits
    const emitCallRegex = /emit\s*\(\s*['"]([^'"]+)['"]/g;
    let match;
    while ((match = emitCallRegex.exec(codeOnly)) !== null) {
      const emitName = match[1];
      if (!declaredEmits.some(e => e.name === emitName)) {
        const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
        issues.push(createIssue(
          'component-emit',
          'cross-file/undeclared-emit',
          'error',
          `Event '${emitName}' is emitted but not declared in defineEmits`,
          filename,
          loc.line,
          loc.column,
          {
            endLine: loc.endLine,
            endColumn: loc.endColumn,
            suggestion: `Add '${emitName}' to defineEmits type definition`,
          }
        ));
      }
    }
  }

  // Check for unhandled event listeners
  for (const [filename, source] of Object.entries(files.value)) {
    const listenerRegex = /@([\w-]+)(?:\.[\w-]+)*="/g;
    let match;
    while ((match = listenerRegex.exec(source)) !== null) {
      const eventName = match[1];
      // Skip native events
      if (isNativeEvent(eventName)) continue;

      // Check if this event is declared by any imported component
      const imports = dependencyGraph.value[filename] || [];
      let hasEmitter = false;
      for (const imp of imports) {
        const impSource = files.value[imp];
        if (impSource && impSource.includes(`'${eventName}'`)) {
          hasEmitter = true;
          break;
        }
      }

      if (!hasEmitter && !['update', 'modelValue'].includes(eventName)) {
        const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
        issues.push(createIssue(
          'component-emit',
          'cross-file/unmatched-listener',
          'info',
          `Listening for @${eventName} but no imported component declares this emit`,
          filename,
          loc.line,
          loc.column,
          { endLine: loc.endLine, endColumn: loc.endColumn }
        ));
      }
    }
  }

  return issues;
}

function analyzeFallthroughAttrs(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  for (const [filename, source] of Object.entries(files.value)) {
    const templateMatch = source.match(/<template>([\s\S]*)<\/template>/);
    if (!templateMatch) continue;

    const template = templateMatch[1];

    // Properly count root elements by tracking tag depth
    const rootElementCount = countRootElements(template);
    const hasMultipleRoots = rootElementCount > 1;

    // Check various ways $attrs can be used
    const attrsUsage = analyzeAttrsUsage(source, template);

    if (hasMultipleRoots && !attrsUsage.bindsExplicitly) {
      // Check if component receives any non-prop attributes from parents
      const componentName = filename.replace('.vue', '');
      let hasPassedAttrs = false;

      for (const [parentFile, parentSource] of Object.entries(files.value)) {
        if (parentFile === filename) continue;
        // Check if parent uses this component with non-standard attributes
        const usageRegex = new RegExp(`<${componentName}[^>]*(?:data-|aria-|class=|style=)[^>]*>`, 'i');
        if (usageRegex.test(parentSource)) {
          hasPassedAttrs = true;
          break;
        }
      }

      const loc = findLineAndColumn(source, /<template[^>]*>/);
      if (loc) {
        // If useAttrs() is used, provide more specific guidance
        if (attrsUsage.usesUseAttrs && !attrsUsage.usesInTemplate) {
          issues.push(createIssue(
            'fallthrough-attrs',
            'cross-file/useattrs-not-bound',
            'warning',
            `useAttrs() is called but attrs are not bound to any element in template`,
            filename,
            loc.line,
            loc.column,
            {
              endLine: loc.endLine,
              endColumn: loc.endColumn,
              suggestion: 'Use v-bind="attrs" or bind specific properties like :class="attrs.class"',
            }
          ));
        } else {
          issues.push(createIssue(
            'fallthrough-attrs',
            'cross-file/multi-root-attrs',
            hasPassedAttrs ? 'warning' : 'info',
            `Component has ${rootElementCount} root elements but $attrs is not explicitly bound`,
            filename,
            loc.line,
            loc.column,
            {
              endLine: loc.endLine,
              endColumn: loc.endColumn,
              suggestion: attrsUsage.usesUseAttrs
                ? 'Bind attrs from useAttrs() to the intended root element'
                : 'Add v-bind="$attrs" to the intended root element, or use useAttrs() composable',
            }
          ));
        }
      }
    }

    // Check for inheritAttrs: false without $attrs usage
    if (source.includes('inheritAttrs: false') || source.includes('inheritAttrs:false')) {
      if (!attrsUsage.usesInTemplate && !attrsUsage.usesUseAttrs) {
        const loc = findLineAndColumn(source, /inheritAttrs\s*:\s*false/);
        if (loc) {
          issues.push(createIssue(
            'fallthrough-attrs',
            'cross-file/inheritattrs-disabled-unused',
            'warning',
            `inheritAttrs is disabled but $attrs is not used anywhere`,
            filename,
            loc.line,
            loc.column,
            {
              suggestion: 'Use v-bind="$attrs", useAttrs(), or $attrs properties in template',
            }
          ));
        }
      }
    }
  }

  return issues;
}

// Analyze how $attrs is used in a component
function analyzeAttrsUsage(source: string, template: string): {
  bindsExplicitly: boolean;      // v-bind="$attrs" or v-bind="attrs"
  usesUseAttrs: boolean;         // useAttrs() composable
  usesInTemplate: boolean;       // $attrs.* or attrs.* in template
  usedProperties: string[];      // Specific properties accessed
} {
  const result = {
    bindsExplicitly: false,
    usesUseAttrs: false,
    usesInTemplate: false,
    usedProperties: [] as string[],
  };

  // Check for useAttrs() in script
  if (/useAttrs\s*\(\s*\)/.test(source)) {
    result.usesUseAttrs = true;
    // Find the variable name: const attrs = useAttrs()
    const useAttrsMatch = source.match(/(?:const|let)\s+(\w+)\s*=\s*useAttrs\s*\(\s*\)/);
    if (useAttrsMatch) {
      const varName = useAttrsMatch[1];
      // Check if this variable is used in template
      const varBindPattern = new RegExp(`v-bind=["']${varName}["']|:=["']${varName}["']`);
      if (varBindPattern.test(template)) {
        result.bindsExplicitly = true;
        result.usesInTemplate = true;
      }
      // Check for property access: attrs.class, attrs.style, etc.
      const propAccessPattern = new RegExp(`${varName}\\.(\\w+)`, 'g');
      let match;
      while ((match = propAccessPattern.exec(template)) !== null) {
        result.usedProperties.push(match[1]);
        result.usesInTemplate = true;
      }
    }
  }

  // Check for direct $attrs usage in template
  if (/v-bind=["']\$attrs["']|:=["']\$attrs["']/.test(template)) {
    result.bindsExplicitly = true;
    result.usesInTemplate = true;
  }

  // Check for $attrs property access in template
  const attrsPropertyPattern = /\$attrs\.(\w+)/g;
  let match;
  while ((match = attrsPropertyPattern.exec(template)) !== null) {
    result.usedProperties.push(match[1]);
    result.usesInTemplate = true;
  }

  // Check for $attrs in interpolation or v-bind
  if (/\{\{\s*\$attrs\b|\$attrs\s*\}\}|:\w+=['"]\$attrs\./.test(template)) {
    result.usesInTemplate = true;
  }

  return result;
}

// Count root-level elements in a template (depth 0 elements only)
function countRootElements(template: string): number {
  // Remove comments first
  const withoutComments = template.replace(/<!--[\s\S]*?-->/g, '');

  // Self-closing void elements that don't need closing tags
  const voidElements = new Set([
    'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input',
    'link', 'meta', 'param', 'source', 'track', 'wbr'
  ]);

  let depth = 0;
  let rootCount = 0;

  // Match all tags (opening, closing, self-closing)
  const tagRegex = /<\/?([a-zA-Z][\w-]*)[^>]*\/?>/g;
  let match;

  while ((match = tagRegex.exec(withoutComments)) !== null) {
    const fullTag = match[0];
    const tagName = match[1].toLowerCase();

    const isClosing = fullTag.startsWith('</');
    const isSelfClosing = fullTag.endsWith('/>') || voidElements.has(tagName);

    if (isClosing) {
      depth--;
    } else {
      if (depth === 0) {
        rootCount++;
      }
      if (!isSelfClosing) {
        depth++;
      }
    }
  }

  return rootCount;
}

function analyzeReactivity(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];

  // Collect all provides with their values to check if they contain refs
  const provideValueIsReactive = new Map<string, boolean>();
  for (const [_filename, source] of Object.entries(files.value)) {
    const result = croquisResults.value[_filename];
    if (!result?.croquis) continue;

    for (const p of result.croquis.provides || []) {
      const keyValue = p.key.type === 'symbol' ? `Symbol:${p.key.value}` : p.key.value;
      // Check if the provide value contains ref() or reactive()
      // This is a simple heuristic based on the value string
      const valueContainsRef = p.value && (
        p.value.includes('ref(') ||
        p.value.includes('reactive(') ||
        /\{\s*\w+\s*:\s*ref\s*\(/.test(p.value)
      );
      provideValueIsReactive.set(keyValue, valueContainsRef);
    }
  }

  for (const [filename, source] of Object.entries(files.value)) {
    const result = croquisResults.value[filename];
    if (!result?.croquis) continue;

    // Note: Direct destructure of inject() is handled by analyzeProvideInject()
    // Here we only check for INDIRECT patterns like: const x = inject(...); const { a } = x;

    // Check for destructuring of inject result variable (indirect pattern)
    // Use bindings info from Rust analysis
    const injectBindings = new Map<string, string>(); // localName -> injectKey
    for (const inj of result.croquis.injects || []) {
      if (inj.localName && inj.pattern === 'simple') {
        const keyValue = inj.key.type === 'symbol' ? `Symbol:${inj.key.value}` : inj.key.value;
        injectBindings.set(inj.localName, keyValue);
      }
    }

    // Look for destructure patterns in source: const { x, y } = injectVar
    // We search the source directly because binding positions aren't reliable
    for (const [injectVar, injectKey] of injectBindings) {
      // Skip if provide value contains refs
      if (provideValueIsReactive.get(injectKey)) continue;

      // Find all lines that match: const { ... } = injectVar
      const destructureRegex = new RegExp(`const\\s*\\{\\s*([^}]+)\\s*\\}\\s*=\\s*${injectVar}\\b`, 'g');
      let match;
      while ((match = destructureRegex.exec(source)) !== null) {
        const propsStr = match[1];
        const props = propsStr.split(',').map(p => p.trim().split(':')[0].trim()).filter(Boolean);
        const matchStart = match.index;
        const loc = findLineAndColumnAtOffset(source, matchStart, match[0].length);

        issues.push(createIssue(
          'reactivity',
          'cross-file/reactivity-loss',
          'error',
          `Destructuring '${injectVar}' (from inject('${injectKey}')) loses reactivity for: ${props.join(', ')}`,
          filename,
          loc.line,
          loc.column,
          {
            endLine: loc.endLine,
            endColumn: loc.endColumn,
            suggestion: `Use toRefs(${injectVar}) or computed(() => ${injectVar}.propName)`,
          }
        ));
      }
    }

    // 3. Check for destructuring of reactive() result
    const reactiveBindings = new Set<string>();
    for (const binding of result.croquis.bindings || []) {
      if (binding.source === 'reactive') {
        reactiveBindings.add(binding.name);
      }
    }

    // Look for destructuring patterns from reactive bindings
    for (const binding of result.croquis.bindings || []) {
      if (binding.source === 'local' && binding.kind === 'SetupConst') {
        const bindingStart = binding.start || 0;
        const lineStart = source.lastIndexOf('\n', bindingStart) + 1;
        const lineEnd = source.indexOf('\n', bindingStart);
        const line = source.substring(lineStart, lineEnd === -1 ? undefined : lineEnd);

        for (const reactiveVar of reactiveBindings) {
          // Skip if toRefs is used
          if (source.includes(`toRefs(${reactiveVar})`)) continue;

          const destructurePattern = new RegExp(`const\\s*\\{[^}]*\\b${binding.name}\\b[^}]*\\}\\s*=\\s*${reactiveVar}\\b`);
          if (destructurePattern.test(line)) {
            const loc = findLineAndColumnAtOffset(source, bindingStart, binding.name.length);
            issues.push(createIssue(
              'reactivity',
              'cross-file/reactivity-loss',
              'warning',
              `Destructuring reactive object '${reactiveVar}' loses reactivity for: ${binding.name}`,
              filename,
              loc.line,
              loc.column,
              {
                endLine: loc.endLine,
                endColumn: loc.endColumn,
                suggestion: `Use toRefs(${reactiveVar}) to maintain reactivity`,
              }
            ));
          }
        }
      }
    }

    // 4. Check Pinia store destructuring (still use regex for this pattern)
    const codeOnly = stripComments(source);
    const storeDestructureRegex = /const\s*\{([^}]+)\}\s*=\s*(\w+Store)\s*\(\s*\)/g;
    let match;
    while ((match = storeDestructureRegex.exec(codeOnly)) !== null) {
      // Check for storeToRefs usage
      if (codeOnly.includes(`storeToRefs(${match[2]}`)) continue;

      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      const props = match[1].split(',').map(p => p.trim().split(':')[0].trim());
      issues.push(createIssue(
        'reactivity',
        'cross-file/store-reactivity-loss',
        'warning',
        `Destructuring Pinia store loses reactivity for: ${props.join(', ')}`,
        filename,
        loc.line,
        loc.column,
        {
          endLine: loc.endLine,
          endColumn: loc.endColumn,
          suggestion: `Use storeToRefs(${match[2]}()) for state and getters`,
        }
      ));
    }
  }

  return issues;
}

function analyzeUniqueIds(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const staticIds: Map<string, Array<{ file: string; line: number; column: number; endLine: number; endColumn: number }>> = new Map();

  for (const [filename, source] of Object.entries(files.value)) {
    // Find static id attributes
    const idRegex = /\bid=["']([^"'${}]+)["']/g;
    let match;
    while ((match = idRegex.exec(source)) !== null) {
      const id = match[1];
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      if (!staticIds.has(id)) staticIds.set(id, []);
      staticIds.get(id)!.push({ file: filename, ...loc });
    }

    // Check for static ids in v-for
    const vforIdRegex = /v-for=[^>]+>\s*[^]*?id=["']([^"'${}]+)["']/g;
    while ((match = vforIdRegex.exec(source)) !== null) {
      const loc = findLineAndColumnAtOffset(source, match.index, match[0].length);
      issues.push(createIssue(
        'unique-id',
        'cross-file/non-unique-id',
        'error',
        `Static id="${match[1]}" inside v-for will create duplicate IDs`,
        filename,
        loc.line,
        loc.column,
        {
          endLine: loc.endLine,
          endColumn: loc.endColumn,
          suggestion: 'Use a dynamic id like :id="`item-${index}`"',
        }
      ));
    }
  }

  // Check for duplicate static IDs across files
  for (const [id, locations] of staticIds.entries()) {
    if (locations.length > 1) {
      const primary = locations[0];
      issues.push(createIssue(
        'unique-id',
        'cross-file/duplicate-id',
        'warning',
        `Element id="${id}" is duplicated in ${locations.length} locations`,
        primary.file,
        primary.line,
        primary.column,
        {
          relatedLocations: locations.slice(1).map(loc => ({
            file: loc.file,
            line: loc.line,
            column: loc.column,
            message: 'Also defined here',
          })),
          suggestion: 'Use unique IDs across your application',
        }
      ));
    }
  }

  return issues;
}

function analyzeSSRBoundary(): CrossFileIssue[] {
  const issues: CrossFileIssue[] = [];
  const browserApis = ['window', 'document', 'navigator', 'localStorage', 'sessionStorage', 'location', 'history'];

  for (const [filename, source] of Object.entries(files.value)) {
    const scriptMatch = source.match(/<script[^>]*>([^]*?)<\/script>/);
    if (!scriptMatch) continue;

    const script = scriptMatch[1];

    for (const api of browserApis) {
      const apiRegex = new RegExp(`\\b${api}\\b`, 'g');
      let match;
      while ((match = apiRegex.exec(script)) !== null) {
        // Check if inside onMounted or other client-only hooks
        const beforeMatch = script.substring(0, match.index);
        const isInClientHook = /on(Mounted|BeforeMount|Updated|BeforeUpdate)\s*\([^)]*$/.test(beforeMatch) ||
                              /onMounted\s*\(\s*(?:async\s*)?\(\)\s*=>\s*\{[^}]*$/.test(beforeMatch);

        if (!isInClientHook) {
          // Calculate position in full source
          const scriptStart = source.indexOf(scriptMatch[1]);
          const fullOffset = scriptStart + match.index;
          const loc = findLineAndColumnAtOffset(source, fullOffset, api.length);
          issues.push(createIssue(
            'ssr-boundary',
            'cross-file/browser-api-ssr',
            'warning',
            `Browser API '${api}' used outside client-only lifecycle hook`,
            filename,
            loc.line,
            loc.column,
            {
              endLine: loc.endLine,
              endColumn: loc.endColumn,
              suggestion: `Move to onMounted() or guard with 'if (import.meta.client)'`,
            }
          ));
        }
      }
    }
  }

  return issues;
}

function isNativeEvent(event: string): boolean {
  return [
    'click', 'dblclick', 'mousedown', 'mouseup', 'mousemove', 'mouseenter', 'mouseleave',
    'keydown', 'keyup', 'keypress', 'focus', 'blur', 'change', 'input', 'submit',
    'scroll', 'resize', 'load', 'error', 'contextmenu', 'wheel',
    'touchstart', 'touchmove', 'touchend', 'drag', 'dragstart', 'dragend', 'drop',
  ].includes(event);
}

// === File Management ===
function addFile() {
  const name = prompt('Enter file name (e.g., NewComponent.vue)');
  if (name && !files.value[name]) {
    files.value[name] = `<script setup lang="ts">\n// ${name}\n<\/script>\n\n<template>\n  <div></div>\n</template>`;
    activeFile.value = name;
  }
}

function removeFile(name: string) {
  if (Object.keys(files.value).length > 1 && confirm(`Delete ${name}?`)) {
    delete files.value[name];
    if (activeFile.value === name) {
      activeFile.value = Object.keys(files.value)[0];
    }
  }
}

function resetProject() {
  const preset = currentPresetData.value;
  files.value = { ...preset.files };
  activeFile.value = Object.keys(preset.files)[0];
  crossFileIssues.value = [];
  selectedIssue.value = null;
}

function selectPreset(presetId: string) {
  currentPreset.value = presetId;
  const preset = PRESETS.find(p => p.id === presetId);
  if (preset) {
    files.value = { ...preset.files };
    activeFile.value = Object.keys(preset.files)[0];
    crossFileIssues.value = [];
    selectedIssue.value = null;
    nextTick(() => analyzeAll());
  }
}

function selectIssue(issue: CrossFileIssue) {
  selectedIssue.value = issue;
  activeFile.value = issue.file;
  // Workaround: directly update currentSource (vite-plugin-vize watch issue)
  const newSource = files.value[issue.file] || '';
  currentSource.value = newSource;
  // Workaround: directly call Monaco setValue (vite-plugin-vize v-model issue)
  monacoEditorRef.value?.setValue(newSource);
}

function selectFile(name: string) {
  activeFile.value = name;
  // Workaround: directly update currentSource (vite-plugin-vize watch issue)
  const newSource = files.value[name] || '';
  currentSource.value = newSource;
  // Workaround: directly call Monaco setValue (vite-plugin-vize v-model issue)
  monacoEditorRef.value?.setValue(newSource);
}

function getFileIcon(filename: string): string {
  if (filename.endsWith('.vue')) return mdiVuejs;
  if (filename.endsWith('.ts')) return mdiLanguageTypescript;
  return mdiFile;
}

function getSeverityIcon(severity: string): string {
  return severity === 'error' ? mdiClose : severity === 'warning' ? mdiAlert : mdiInformation;
}

function getTypeLabel(type: string): string {
  const labels: Record<string, string> = {
    'provide-inject': 'Provide/Inject',
    'component-emit': 'Component Emit',
    'fallthrough-attrs': 'Fallthrough Attrs',
    'reactivity': 'Reactivity',
    'unique-id': 'Unique ID',
    'ssr-boundary': 'SSR Boundary',
  };
  return labels[type] || type;
}

function getTypeColor(type: string): string {
  const colors: Record<string, string> = {
    'provide-inject': '#8b5cf6',
    'component-emit': '#f59e0b',
    'fallthrough-attrs': '#06b6d4',
    'reactivity': '#ef4444',
    'unique-id': '#10b981',
    'ssr-boundary': '#3b82f6',
  };
  return colors[type] || '#6b7280';
}

// === Watchers ===
let analyzeTimeout: ReturnType<typeof setTimeout> | null = null;

function debouncedAnalyze() {
  if (analyzeTimeout) clearTimeout(analyzeTimeout);
  analyzeTimeout = setTimeout(() => {
    analyzeAll();
  }, 300);
}

watch([files, options], () => {
  debouncedAnalyze();
}, { deep: true });

// Workaround for vite-plugin-vize prop reactivity issue
// Use getWasm() directly with polling instead of props.compiler
let hasCompilerInitialized = false;
let pollInterval: ReturnType<typeof setInterval> | null = null;

function tryInitialize() {
  const compiler = getWasm();
  if (compiler && !hasCompilerInitialized) {
    hasCompilerInitialized = true;
    if (pollInterval) {
      clearInterval(pollInterval);
      pollInterval = null;
    }
    analyzeAll();
  }
}

onMounted(() => {
  // Try to initialize immediately if compiler is available
  tryInitialize();

  // If not, poll for it
  if (!hasCompilerInitialized) {
    pollInterval = setInterval(tryInitialize, 100);
    // Stop polling after 10 seconds
    setTimeout(() => {
      if (pollInterval) {
        clearInterval(pollInterval);
        pollInterval = null;
      }
    }, 10000);
  }
});

onUnmounted(() => {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
});
</script>

<template>
  <div ref="containerRef" class="cross-file-playground" :style="gridStyle" :class="{ resizing: isResizingSidebar || isResizingDiagnostics }">
    <!-- Sidebar: File Tree & Dependency Graph -->
    <aside class="sidebar">
      <!-- Preset Selector -->
      <div class="sidebar-section preset-section">
        <div class="section-header">
          <h3>Presets</h3>
        </div>
        <div class="preset-list">
          <button
            v-for="preset in PRESETS"
            :key="preset.id"
            :class="['preset-item', { active: currentPreset === preset.id }]"
            @click="selectPreset(preset.id)"
            :title="preset.description"
          >
            <svg class="preset-icon" viewBox="0 0 24 24"><path :d="preset.icon" fill="currentColor" /></svg>
            <span class="preset-name">{{ preset.name }}</span>
          </button>
        </div>
      </div>

      <div class="sidebar-section">
        <div class="section-header">
          <h3>Project Files</h3>
          <div class="section-actions">
            <button @click="addFile" class="icon-btn" title="Add file">+</button>
            <button @click="resetProject" class="icon-btn" title="Reset">↺</button>
          </div>
        </div>
        <nav class="file-tree">
          <div
            v-for="name in fileNames"
            :key="name"
            :class="['file-item', { active: activeFile === name, 'has-errors': issuesByFile[name]?.some(i => i.severity === 'error'), 'has-warnings': issuesByFile[name]?.some(i => i.severity === 'warning') }]"
            @click="selectFile(name)"
          >
            <svg class="file-icon" viewBox="0 0 24 24"><path :d="getFileIcon(name)" fill="currentColor" /></svg>
            <span class="file-name">{{ name }}</span>
            <span v-if="issuesByFile[name]?.length" class="file-badge" :class="issuesByFile[name].some(i => i.severity === 'error') ? 'error' : 'warning'">
              <span class="badge-count">{{ issuesByFile[name].length }}</span>
            </span>
            <button v-if="fileNames.length > 1" @click.stop="removeFile(name)" class="file-delete">×</button>
          </div>
        </nav>
      </div>

      <div class="sidebar-section">
        <div class="section-header">
          <h3>Dependencies</h3>
        </div>
        <div class="dependency-graph">
          <div v-for="(deps, file) in dependencyGraph" :key="file" class="dep-node">
            <span class="dep-file">{{ file }}</span>
            <div v-if="deps.length" class="dep-arrows">
              <div v-for="dep in deps" :key="dep" class="dep-edge">
                <span class="dep-arrow">→</span>
                <span class="dep-target" @click="selectFile(dep)">{{ dep }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="sidebar-section options-section">
        <div class="section-header">
          <h3>Analyzers</h3>
          <span class="analysis-mode-badge" title="Strict Static Analysis: No heuristics, all issues are based on precise AST analysis">STRICT</span>
        </div>
        <div class="options-grid">
          <label class="option-toggle">
            <input type="checkbox" v-model="options.provideInject" />
            <span>Provide/Inject</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.componentEmits" />
            <span>Component Emits</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.fallthroughAttrs" />
            <span>Fallthrough Attrs</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.reactivityTracking" />
            <span>Reactivity</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.uniqueIds" />
            <span>Unique IDs</span>
          </label>
          <label class="option-toggle">
            <input type="checkbox" v-model="options.serverClientBoundary" />
            <span>SSR Boundary</span>
          </label>
        </div>
      </div>
    </aside>

    <!-- Resize Handle: Sidebar -->
    <div class="resize-handle resize-handle-left" @mousedown="startSidebarResize"></div>

    <!-- Main Content: Editor -->
    <main class="editor-pane">
      <div class="editor-header">
        <div class="editor-tabs">
          <button
            v-for="name in fileNames"
            :key="name"
            :class="['editor-tab', { active: activeFile === name }]"
            @click="selectFile(name)"
          >
            <svg class="tab-icon" viewBox="0 0 24 24"><path :d="getFileIcon(name)" fill="currentColor" /></svg>
            <span class="tab-name">{{ name }}</span>
            <span v-if="issuesByFile[name]?.length" class="tab-badge" :class="issuesByFile[name].some(i => i.severity === 'error') ? 'error' : 'warning'">
              <span class="badge-count">{{ issuesByFile[name].length }}</span>
            </span>
          </button>
        </div>
        <div class="editor-status">
          <span v-if="isAnalyzing" class="status-analyzing">Analyzing...</span>
          <span v-else class="status-time">{{ analysisTime.toFixed(1) }}ms</span>
        </div>
      </div>
      <div class="editor-content">
        <MonacoEditor
          ref="monacoEditorRef"
          v-model="currentSource"
          :language="editorLanguage"
          :diagnostics="currentDiagnostics"
        />
      </div>
    </main>

    <!-- Resize Handle: Diagnostics -->
    <div class="resize-handle resize-handle-right" @mousedown="startDiagnosticsResize"></div>

    <!-- Right Panel: Diagnostics -->
    <aside class="diagnostics-pane">
      <div class="diagnostics-header">
        <h3>Diagnostics</h3>
        <div class="diagnostics-stats">
          <span class="stat-chip error" v-if="stats.errors">{{ stats.errors }} errors</span>
          <span class="stat-chip warning" v-if="stats.warnings">{{ stats.warnings }} warnings</span>
          <span class="stat-chip info" v-if="stats.infos">{{ stats.infos }} info</span>
        </div>
      </div>

      <div v-if="crossFileIssues.length === 0" class="diagnostics-empty">
        <svg class="empty-icon" viewBox="0 0 24 24"><path :d="mdiCheck" fill="currentColor" /></svg>
        <span>No issues detected</span>
      </div>

      <div v-else class="diagnostics-list">
        <!-- Group by type -->
        <div v-for="(issues, type) in issuesByType" :key="type" class="issue-group">
          <div class="group-header" :style="{ '--type-color': getTypeColor(type) }">
            <span class="group-badge">{{ getTypeLabel(type) }}</span>
            <span class="group-count">{{ issues.length }}</span>
          </div>
          <div class="group-issues">
            <div
              v-for="issue in issues"
              :key="issue.id"
              :class="['issue-card', issue.severity, { selected: selectedIssue?.id === issue.id }]"
              @click="selectIssue(issue)"
            >
              <div class="issue-header">
                <svg class="severity-icon" viewBox="0 0 24 24"><path :d="getSeverityIcon(issue.severity)" fill="currentColor" /></svg>
                <span class="issue-code">{{ issue.code }}</span>
                <span class="issue-location">{{ issue.file }}:{{ issue.line }}</span>
              </div>
              <div class="issue-message">{{ issue.message }}</div>
              <div v-if="issue.suggestion" class="issue-suggestion">
                <span class="suggestion-icon">→</span>
                <span class="suggestion-text">{{ issue.suggestion }}</span>
              </div>
              <div v-if="issue.relatedLocations?.length" class="issue-related">
                <div v-for="(rel, i) in issue.relatedLocations" :key="i" class="related-item">
                  <span class="related-loc">{{ rel.file }}:{{ rel.line }}</span>
                  <span class="related-msg">{{ rel.message }}</span>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </aside>
  </div>
</template>

<style scoped>
.cross-file-playground {
  display: grid;
  grid-template-columns: 220px 4px 1fr 4px 320px;
  grid-column: 1 / -1;
  height: 100%;
  min-height: 0;
  background: var(--bg-primary);
  font-size: 12px;
  user-select: none;
}

.cross-file-playground.resizing {
  cursor: col-resize;
}

.cross-file-playground.resizing * {
  pointer-events: none;
}

/* === Resize Handles === */
.resize-handle {
  width: 4px;
  background: var(--border-primary);
  cursor: col-resize;
  transition: background 0.15s;
  position: relative;
}

.resize-handle:hover,
.resize-handle:active {
  background: var(--accent-primary);
}

.resize-handle::after {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  width: 8px;
  left: -2px;
}

/* === Sidebar === */
.sidebar {
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border-primary);
  overflow: hidden;
}

.sidebar-section {
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.sidebar-section:not(:last-child) {
  border-bottom: 1px solid var(--border-primary);
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-tertiary);
}

.section-header h3 {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
  margin: 0;
}

.analysis-mode-badge {
  font-size: 9px;
  font-weight: 700;
  padding: 2px 6px;
  border-radius: 3px;
  background: linear-gradient(135deg, #10b981, #059669);
  color: #fff;
  letter-spacing: 0.5px;
  cursor: help;
}

/* Preset Selector */
.preset-section {
  flex-shrink: 0;
}

.preset-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 8px;
}

.preset-item {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 10px;
  background: var(--bg-primary);
  border: 1px solid var(--border-primary);
  border-radius: 6px;
  cursor: pointer;
  font-size: 10px;
  color: var(--text-secondary);
  transition: all 0.15s;
  flex: 1;
  min-width: calc(50% - 4px);
  justify-content: flex-start;
}

.preset-item:hover {
  background: var(--bg-tertiary);
  border-color: var(--text-muted);
  color: var(--text-primary);
}

.preset-item.active {
  background: rgba(224, 112, 72, 0.15);
  border-color: var(--accent-rust);
  color: var(--accent-rust);
}

.preset-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.preset-name {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.section-actions {
  display: flex;
  gap: 4px;
}

.icon-btn {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid var(--border-primary);
  border-radius: 3px;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}

.icon-btn:hover {
  background: var(--bg-primary);
  color: var(--text-primary);
  border-color: var(--text-muted);
}

/* File Tree */
.file-tree {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  cursor: pointer;
  transition: background 0.1s;
  position: relative;
}

.file-item:hover {
  background: var(--bg-tertiary);
}

.file-item.active {
  background: var(--accent-primary);
  background: rgba(224, 112, 72, 0.15);
}

.file-item.has-errors .file-icon { color: #ef4444; }
.file-item.has-warnings .file-icon { color: #f59e0b; }

.file-icon {
  width: 12px;
  height: 12px;
  color: var(--accent-rust);
  flex-shrink: 0;
}

.file-name {
  flex: 1;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.file-badge {
  font-size: 9px;
  padding: 1px 5px;
  border-radius: 8px;
  font-weight: 600;
}

.file-badge.error {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.file-badge.warning {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.file-delete {
  position: absolute;
  right: 8px;
  width: 16px;
  height: 16px;
  display: none;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 14px;
  border-radius: 2px;
}

.file-item:hover .file-delete {
  display: flex;
}

.file-delete:hover {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

/* Dependency Graph */
.dependency-graph {
  padding: 8px 12px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
}

.dep-node {
  margin-bottom: 8px;
}

.dep-file {
  color: var(--text-secondary);
}

.dep-arrows {
  padding-left: 12px;
  margin-top: 2px;
}

.dep-edge {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--text-muted);
}

.dep-arrow {
  color: var(--accent-rust);
}

.dep-target {
  color: var(--text-secondary);
  cursor: pointer;
}

.dep-target:hover {
  color: var(--accent-rust);
  text-decoration: underline;
}

/* Options */
.options-section {
  margin-top: auto;
}

.options-grid {
  padding: 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.option-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 11px;
  color: var(--text-secondary);
}

.option-toggle input {
  width: 12px;
  height: 12px;
  accent-color: var(--accent-primary);
}

.option-toggle:hover {
  color: var(--text-primary);
}

/* === Editor Pane === */
.editor-pane {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
}

.editor-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border-primary);
  padding-right: 12px;
}

.editor-tabs {
  display: flex;
  overflow-x: auto;
}

.editor-tab {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  background: transparent;
  border: none;
  border-right: 1px solid var(--border-primary);
  font-size: 11px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-muted);
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.1s;
}

.editor-tab:hover {
  background: var(--bg-tertiary);
  color: var(--text-secondary);
}

.editor-tab.active {
  background: var(--bg-primary);
  color: var(--text-primary);
  border-bottom: 2px solid var(--accent-rust);
  margin-bottom: -1px;
}

.tab-icon {
  width: 12px;
  height: 12px;
  color: var(--accent-rust);
  flex-shrink: 0;
}

.tab-badge {
  font-size: 9px;
  padding: 1px 5px;
  border-radius: 8px;
  font-weight: 600;
}

.tab-badge.error {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.tab-badge.warning {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.editor-status {
  font-size: 10px;
  font-family: 'JetBrains Mono', monospace;
}

.status-analyzing {
  color: var(--accent-rust);
}

.status-time {
  color: var(--text-muted);
}

.editor-content {
  flex: 1;
  min-height: 0;
}

/* === Diagnostics Pane === */
.diagnostics-pane {
  display: flex;
  flex-direction: column;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border-primary);
  overflow: hidden;
}

.diagnostics-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-tertiary);
  border-bottom: 1px solid var(--border-primary);
}

.diagnostics-header h3 {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-muted);
  margin: 0;
}

.diagnostics-stats {
  display: flex;
  gap: 6px;
}

.stat-chip {
  font-size: 9px;
  padding: 2px 6px;
  border-radius: 3px;
  font-weight: 600;
}

.stat-chip.error {
  background: rgba(239, 68, 68, 0.2);
  color: #f87171;
}

.stat-chip.warning {
  background: rgba(251, 191, 36, 0.2);
  color: #fbbf24;
}

.stat-chip.info {
  background: rgba(96, 165, 250, 0.2);
  color: #60a5fa;
}

.diagnostics-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 32px;
  color: #4ade80;
}

.empty-icon {
  width: 24px;
  height: 24px;
}

.diagnostics-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}

.issue-group {
  margin-bottom: 12px;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  margin-bottom: 4px;
  background: var(--bg-tertiary);
  border-radius: 4px;
  border-left: 3px solid var(--type-color, var(--text-muted));
}

.group-badge {
  font-size: 10px;
  font-weight: 600;
  color: var(--text-secondary);
}

.group-count {
  font-size: 10px;
  color: var(--text-muted);
  font-family: 'JetBrains Mono', monospace;
}

.group-issues {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.issue-card {
  padding: 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border-primary);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.1s;
  border-left: 3px solid transparent;
}

.issue-card:hover {
  background: var(--bg-tertiary);
}

.issue-card.selected {
  border-color: var(--accent-rust);
  background: rgba(224, 112, 72, 0.1);
}

.issue-card.error { border-left-color: #ef4444; }
.issue-card.warning { border-left-color: #f59e0b; }
.issue-card.info { border-left-color: #60a5fa; }

.issue-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 4px;
}

.severity-icon {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
}

.issue-card.error .severity-icon { color: #ef4444; }
.issue-card.warning .severity-icon { color: #f59e0b; }
.issue-card.info .severity-icon { color: #60a5fa; }

.issue-code {
  font-size: 9px;
  font-family: 'JetBrains Mono', monospace;
  padding: 1px 4px;
  background: var(--bg-secondary);
  border-radius: 2px;
  color: var(--text-muted);
}

.issue-location {
  margin-left: auto;
  font-size: 9px;
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-muted);
}

.issue-message {
  font-size: 11px;
  color: var(--text-primary);
  line-height: 1.4;
}

.issue-suggestion {
  margin-top: 6px;
  padding: 6px;
  font-size: 10px;
  color: #4ade80;
  background: rgba(74, 222, 128, 0.1);
  border-radius: 3px;
  display: flex;
  gap: 6px;
}

.suggestion-icon {
  flex-shrink: 0;
}

.issue-related {
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--border-primary);
}

.related-item {
  display: flex;
  gap: 8px;
  font-size: 10px;
  color: var(--text-muted);
  margin-bottom: 2px;
}

.related-loc {
  font-family: 'JetBrains Mono', monospace;
  color: var(--text-secondary);
}

/* === Responsive === */
@media (max-width: 1200px) {
  .cross-file-playground {
    grid-template-columns: 180px 1fr 280px;
  }
}

@media (max-width: 900px) {
  .cross-file-playground {
    grid-template-columns: 1fr;
    grid-template-rows: auto 1fr auto;
  }

  .sidebar {
    flex-direction: row;
    border-right: none;
    border-bottom: 1px solid var(--border-primary);
    overflow-x: auto;
  }

  .sidebar-section {
    flex-direction: row;
    min-width: max-content;
  }

  .diagnostics-pane {
    border-left: none;
    border-top: 1px solid var(--border-primary);
    max-height: 300px;
  }
}
</style>
