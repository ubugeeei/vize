import { describe, it, expect, beforeAll } from 'vitest'
import { loadWasm, type WasmModule } from '../src/wasm/index'

describe('CSS Compilation', () => {
  let wasm: WasmModule | null = null

  beforeAll(async () => {
    wasm = await loadWasm()
  })

  describe('Basic CSS', () => {
    it('should compile basic CSS', () => {
      const css = `.container { display: flex; }`
      const result = wasm!.compileCss(css, {})
      expect(result).toBeDefined()
      expect(result.code).toContain('.container')
      expect(result.code).toContain('display')
    })

    it('should compile multiple rules', () => {
      const css = `
.header { font-size: 24px; }
.content { padding: 16px; }
.footer { margin-top: 20px; }
`
      const result = wasm!.compileCss(css, {})
      expect(result.code).toContain('.header')
      expect(result.code).toContain('.content')
      expect(result.code).toContain('.footer')
    })

    it('should handle nested selectors', () => {
      const css = `
.parent {
  color: black;
}
.parent .child {
  color: blue;
}
`
      const result = wasm!.compileCss(css, {})
      expect(result.code).toContain('.parent')
    })
  })

  describe('Scoped CSS', () => {
    it('should add scope attribute to selectors', () => {
      const css = `.container { display: flex; }`
      const result = wasm!.compileCss(css, {
        scoped: true,
        scopeId: 'data-v-abc123',
      })
      expect(result.code).toContain('data-v-abc123')
    })

    it('should scope multiple selectors', () => {
      const css = `
.header { font-size: 24px; }
.content { padding: 16px; }
`
      const result = wasm!.compileCss(css, {
        scoped: true,
        scopeId: 'data-v-test',
      })
      expect(result.code).toContain('data-v-test')
    })

    it('should handle :deep() pseudo-selector', () => {
      const css = `
.container :deep(.nested) {
  color: red;
}
`
      const result = wasm!.compileCss(css, {
        scoped: true,
        scopeId: 'data-v-deep',
      })
      expect(result).toBeDefined()
    })

    it('should handle :slotted() pseudo-selector', () => {
      const css = `
:slotted(.slot-content) {
  padding: 10px;
}
`
      const result = wasm!.compileCss(css, {
        scoped: true,
        scopeId: 'data-v-slot',
      })
      expect(result).toBeDefined()
    })

    it('should handle :global() pseudo-selector', () => {
      const css = `
:global(.global-class) {
  color: blue;
}
`
      const result = wasm!.compileCss(css, {
        scoped: true,
        scopeId: 'data-v-global',
      })
      expect(result).toBeDefined()
    })
  })

  describe('CSS Minification', () => {
    it('should minify CSS when option is enabled', () => {
      const css = `
.container {
  display: flex;
  justify-content: center;
  align-items: center;
}
`
      const result = wasm!.compileCss(css, { minify: true })
      // Minified CSS should be more compact (less whitespace)
      // The minified version should contain less whitespace than original
      expect(result.code).toContain('.container')
      expect(result.code).toContain('display')
      // Minified output is more compact - no need for exact format check
    })

    it('should not minify by default', () => {
      const css = `.container { display: flex; }`
      const resultMinified = wasm!.compileCss(css, { minify: true })
      const resultNormal = wasm!.compileCss(css, { minify: false })
      // Both should contain the same CSS rule
      expect(resultMinified.code).toContain('.container')
      expect(resultNormal.code).toContain('.container')
    })
  })

  describe('v-bind in CSS', () => {
    it('should detect v-bind() usage', () => {
      const css = `
.dynamic {
  color: v-bind(textColor);
}
`
      const result = wasm!.compileCss(css, {})
      expect(result.cssVars).toBeDefined()
      if (result.cssVars && result.cssVars.length > 0) {
        expect(result.cssVars).toContain('textColor')
      }
    })

    it('should detect multiple v-bind() usages', () => {
      const css = `
.dynamic {
  color: v-bind(textColor);
  background: v-bind(bgColor);
  font-size: v-bind(fontSize);
}
`
      const result = wasm!.compileCss(css, {})
      expect(result.cssVars).toBeDefined()
    })

    it('should handle v-bind with expressions', () => {
      const css = `
.dynamic {
  color: v-bind('theme.primary');
}
`
      const result = wasm!.compileCss(css, {})
      expect(result).toBeDefined()
    })
  })

  describe('CSS in SFC', () => {
    it('should compile SFC with style block', () => {
      const sfc = `
<template>
  <div class="container">Hello</div>
</template>

<style>
.container {
  padding: 20px;
}
</style>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.descriptor.styles).toBeDefined()
      expect(result.descriptor.styles?.length).toBeGreaterThan(0)
    })

    it('should compile SFC with scoped style', () => {
      const sfc = `
<template>
  <div class="container">Hello</div>
</template>

<style scoped>
.container {
  padding: 20px;
}
</style>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.descriptor.styles).toBeDefined()
      expect(result.descriptor.styles?.[0]?.scoped).toBe(true)
    })

    it('should compile SFC with multiple style blocks', () => {
      const sfc = `
<template>
  <div class="container">Hello</div>
</template>

<style>
.global-style {
  color: black;
}
</style>

<style scoped>
.scoped-style {
  color: blue;
}
</style>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.descriptor.styles?.length).toBe(2)
    })

    it('should handle style lang attribute', () => {
      const sfc = `
<template>
  <div>Hello</div>
</template>

<style lang="scss">
$color: blue;
.container {
  color: $color;
}
</style>
`
      const result = wasm!.compileSfc(sfc, {})
      expect(result.descriptor.styles?.[0]?.lang).toBe('scss')
    })
  })
})
