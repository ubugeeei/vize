import { describe, it, expect, beforeAll } from 'vitest'
import { loadWasm, type WasmModule } from '../src/wasm/index'

describe('Edge Cases', () => {
  let wasm: WasmModule | null = null

  beforeAll(async () => {
    wasm = await loadWasm()
  })

  describe('Empty/Minimal SFC', () => {
    it('should handle empty template', () => {
      const sfc = `
<template></template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })

    it('should handle template with only whitespace', () => {
      const sfc = `
<template>

</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })

    it('should handle empty script setup', () => {
      const sfc = `
<script setup>
</script>

<template>
  <div>Hello</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })
  })

  describe('Complex TypeScript', () => {
    it('should handle generic function declarations', () => {
      const sfc = `
<script setup lang="ts">
function identity<T>(value: T): T {
  return value
}
const result = identity<string>('test')
</script>

<template>
  <div>{{ result }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      const code = result.script?.code
      expect(code).toBeDefined()
      // The compiled code should contain the function and call
      expect(code).toContain('identity')
      expect(code).toContain('result')
    })

    it('should handle async/await with types', () => {
      const sfc = `
<script setup lang="ts">
async function fetchData(): Promise<string> {
  return 'data'
}
const data = await fetchData()
</script>

<template>
  <div>{{ data }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      const code = result.script?.code
      expect(code).toBeDefined()
      // The compiled code should contain async function
      expect(code).toContain('fetchData')
      expect(code).toContain('data')
    })

    it('should handle type assertions', () => {
      const sfc = `
<script setup lang="ts">
const element = document.querySelector('.test') as HTMLDivElement
const value = (someValue as unknown) as number
</script>

<template>
  <div>Test</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      const code = result.script?.code
      expect(code).toBeDefined()
      // The compiled code should contain the variables
      expect(code).toContain('element')
      expect(code).toContain('document.querySelector')
    })

    it('should handle non-null assertions', () => {
      const sfc = `
<script setup lang="ts">
const element = document.querySelector('.test')!
const value = maybeNull!.property
</script>

<template>
  <div>Test</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })

    it('should handle optional chaining with types', () => {
      const sfc = `
<script setup lang="ts">
interface User { name?: { first: string } }
const user: User = {}
const name = user?.name?.first
</script>

<template>
  <div>{{ name }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
      // TypeScript is preserved in output (auto-detected from lang="ts")
      expect(result.script?.code).toContain('interface User')
    })

    it('should handle enum declarations', () => {
      const sfc = `
<script setup lang="ts">
enum Status {
  Active = 'active',
  Inactive = 'inactive'
}
const status = Status.Active
</script>

<template>
  <div>{{ status }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })

    it('should handle const enum declarations', () => {
      const sfc = `
<script setup lang="ts">
const enum Direction {
  Up,
  Down,
  Left,
  Right
}
const dir = Direction.Up
</script>

<template>
  <div>{{ dir }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })

    it('should handle class with decorators pattern', () => {
      const sfc = `
<script setup lang="ts">
class MyClass {
  private name: string
  constructor(name: string) {
    this.name = name
  }
  public getName(): string {
    return this.name
  }
}
const instance = new MyClass('test')
</script>

<template>
  <div>{{ instance.getName() }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      const code = result.script?.code
      expect(code).toBeDefined()
      // The compiled code should contain the class
      expect(code).toContain('MyClass')
      expect(code).toContain('instance')
      expect(code).toContain('constructor')
    })
  })

  describe('Complex Templates', () => {
    it('should handle deeply nested templates', () => {
      const sfc = `
<script setup>
const items = [[1, 2], [3, 4], [5, 6]]
</script>

<template>
  <div>
    <div v-for="(row, i) in items" :key="i">
      <span v-for="(item, j) in row" :key="j">
        {{ item }}
      </span>
    </div>
  </div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.template?.code).toBeDefined()
    })

    it('should handle multiple root elements (fragments)', () => {
      const sfc = `
<script setup>
</script>

<template>
  <header>Header</header>
  <main>Main</main>
  <footer>Footer</footer>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })

    it('should handle component self-reference', () => {
      const sfc = `
<script setup>
defineProps(['depth'])
</script>

<template>
  <div>
    Depth: {{ depth }}
    <template v-if="depth > 0">
      <component :is="$options" :depth="depth - 1" />
    </template>
  </div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })

    it('should handle teleport', () => {
      const sfc = `
<script setup>
</script>

<template>
  <Teleport to="body">
    <div class="modal">Modal content</div>
  </Teleport>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })

    it('should handle suspense', () => {
      const sfc = `
<script setup>
import AsyncComponent from './AsyncComponent.vue'
</script>

<template>
  <Suspense>
    <template #default>
      <AsyncComponent />
    </template>
    <template #fallback>
      <div>Loading...</div>
    </template>
  </Suspense>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })

    it('should handle keep-alive', () => {
      const sfc = `
<script setup>
import { ref } from 'vue'
const currentTab = ref('A')
</script>

<template>
  <KeepAlive>
    <component :is="currentTab" />
  </KeepAlive>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })
  })

  describe('Special Characters', () => {
    it('should handle template literals', () => {
      const sfc = `
<script setup>
const name = 'World'
const greeting = \`Hello, \${name}!\`
</script>

<template>
  <div>{{ greeting }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })

    it('should handle unicode in template', () => {
      const sfc = `
<script setup>
const emoji = '🎉'
const japanese = 'こんにちは'
</script>

<template>
  <div>{{ emoji }} {{ japanese }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })

    it('should handle special HTML entities', () => {
      const sfc = `
<template>
  <div>&lt;div&gt; &amp; &quot;test&quot;</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result).toBeDefined()
    })
  })

  describe('defineModel', () => {
    it('should handle defineModel', () => {
      const sfc = `
<script setup lang="ts">
const modelValue = defineModel<string>()
</script>

<template>
  <input :value="modelValue" @input="modelValue = $event.target.value" />
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })

    it('should handle defineModel with options', () => {
      const sfc = `
<script setup lang="ts">
const count = defineModel<number>('count', { default: 0 })
</script>

<template>
  <div>{{ count }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })
  })

  describe('defineExpose', () => {
    it('should handle defineExpose', () => {
      const sfc = `
<script setup>
import { ref } from 'vue'
const count = ref(0)
const increment = () => count.value++

defineExpose({
  count,
  increment
})
</script>

<template>
  <div>{{ count }}</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })
  })

  describe('defineOptions', () => {
    it('should handle defineOptions', () => {
      const sfc = `
<script setup>
defineOptions({
  name: 'MyComponent',
  inheritAttrs: false
})
</script>

<template>
  <div>Hello</div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })
  })

  describe('defineSlots', () => {
    it('should handle defineSlots with types', () => {
      const sfc = `
<script setup lang="ts">
const slots = defineSlots<{
  default(props: { msg: string }): any
  header(): any
}>()
</script>

<template>
  <div>
    <slot name="header"></slot>
    <slot :msg="'hello'"></slot>
  </div>
</template>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.script?.code).toBeDefined()
    })
  })
})
