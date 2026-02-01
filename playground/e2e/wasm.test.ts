import { describe, it, expect } from 'vitest'
import { loadWasm, isWasmLoaded, isUsingMock, getWasm } from '../src/wasm/index'

describe('WASM Module', () => {
  it('should load WASM module', async () => {
    const wasm = await loadWasm()
    expect(wasm).toBeDefined()
    expect(isWasmLoaded()).toBe(true)
  })

  it('should return WASM module after loading', () => {
    const wasm = getWasm()
    expect(wasm).not.toBeNull()
  })

  it('should have compileSfc function', () => {
    const wasm = getWasm()
    expect(wasm).not.toBeNull()
    if (wasm) {
      expect(typeof wasm.compileSfc).toBe('function')
    }
  })

  it('should compile a simple SFC', async () => {
    const wasm = getWasm()
    expect(wasm).not.toBeNull()
    if (wasm) {
      const sfc = `
<template>
  <div>Hello</div>
</template>

<script setup>
const msg = 'Hello'
</script>
`
      const result = wasm.compileSfc(sfc, {})
      expect(result).toBeDefined()
      expect(result.descriptor).toBeDefined()
    }
  })

  it('should use real WASM, not mock', () => {
    const usingMock = isUsingMock()
    console.log('Using mock:', usingMock)
    expect(usingMock).toBe(false)
  })
})
