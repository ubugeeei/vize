import { ref, computed } from 'vue'
import { useMessageListener } from './usePostMessage'

export interface RawEventData {
  type: string
  bubbles: boolean
  cancelable: boolean
  composed: boolean
  defaultPrevented: boolean
  eventPhase: number
  isTrusted: boolean
  timeStamp: number
  // Mouse/Pointer
  clientX?: number
  clientY?: number
  screenX?: number
  screenY?: number
  pageX?: number
  pageY?: number
  offsetX?: number
  offsetY?: number
  button?: number
  buttons?: number
  altKey?: boolean
  ctrlKey?: boolean
  metaKey?: boolean
  shiftKey?: boolean
  // Keyboard
  key?: string
  code?: string
  repeat?: boolean
  // Input
  inputType?: string
  data?: string | null
  // Wheel
  deltaX?: number
  deltaY?: number
  deltaZ?: number
  deltaMode?: number
}

export interface ActionEvent {
  name: string
  target?: string
  value?: unknown
  args?: unknown
  timestamp: number
  source: 'dom' | 'vue'
  rawEvent?: RawEventData
  variantName?: string
}

const events = ref<ActionEvent[]>([])
const currentVariant = ref<string>('')

export function useActions() {
  function init() {
    useMessageListener('musea:event', (payload) => {
      const event = payload as ActionEvent
      events.value.push(event)
      // Keep max 200 events
      if (events.value.length > 200) {
        events.value = events.value.slice(-200)
      }
    })
  }

  function clear() {
    events.value = []
  }

  function setCurrentVariant(variantName: string) {
    currentVariant.value = variantName
  }

  const filteredEvents = computed(() => {
    if (!currentVariant.value) return events.value
    return events.value.filter(e => e.variantName === currentVariant.value)
  })

  return {
    events: filteredEvents,
    allEvents: events,
    currentVariant,
    init,
    clear,
    setCurrentVariant,
  }
}
