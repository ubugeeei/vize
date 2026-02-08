import { ref, shallowRef } from 'vue'
import { useMessageListener, sendMessage } from './usePostMessage'

export interface A11yNode {
  html: string
  target: string[]
  failureSummary?: string
}

export interface A11yViolation {
  id: string
  impact: 'critical' | 'serious' | 'moderate' | 'minor'
  description: string
  helpUrl: string
  nodes: A11yNode[]
}

export interface A11yResult {
  violations: A11yViolation[]
  passes: number
  incomplete: number
  error?: string
}

// Singleton state
const results = shallowRef<Map<string, A11yResult>>(new Map())
const isRunning = ref(false)
const currentKey = ref<string>('')

export function useA11y() {
  function init() {
    useMessageListener('musea:a11y-result', (payload: A11yResult) => {
      if (currentKey.value) {
        const newMap = new Map(results.value)
        newMap.set(currentKey.value, payload)
        results.value = newMap
      }
      isRunning.value = false
    })
  }

  function runA11y(iframe: HTMLIFrameElement, key: string) {
    if (isRunning.value) return
    isRunning.value = true
    currentKey.value = key
    sendMessage(iframe, 'musea:run-a11y', {})
  }

  function getResult(key: string): A11yResult | undefined {
    return results.value.get(key)
  }

  function clearResults() {
    results.value = new Map()
  }

  return {
    results,
    isRunning,
    init,
    runA11y,
    getResult,
    clearResults,
  }
}
