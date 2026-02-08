import { ref, computed, shallowRef } from 'vue'

// Singleton state - shared across all uses
const events = shallowRef<CapturedEvent[]>([])
const variantEvents = ref<Map<string, CapturedEvent[]>>(new Map())
const filterType = ref<string>('')
const isPaused = ref(false)
const currentVariantId = ref<string>('')

export interface CapturedEvent {
  id: string
  timestamp: number
  type: string
  target?: string
  payload?: unknown
  variantId: string
  // Raw DOM event properties
  rawEvent?: {
    bubbles?: boolean
    cancelable?: boolean
    composed?: boolean
    defaultPrevented?: boolean
    eventPhase?: number
    isTrusted?: boolean
    timeStamp?: number
    // Mouse event properties
    clientX?: number
    clientY?: number
    button?: number
    buttons?: number
    altKey?: boolean
    ctrlKey?: boolean
    metaKey?: boolean
    shiftKey?: boolean
    // Keyboard event properties
    key?: string
    code?: string
    repeat?: boolean
    // Input event properties
    inputType?: string
    data?: string | null
    // Focus event properties
    relatedTarget?: string | null
  }
}

export interface EventCaptureOptions {
  maxEvents?: number
}

export function useEventCapture(options: EventCaptureOptions = {}) {
  const { maxEvents = 1000 } = options

  // Events filtered by current variant (if set) and type
  const filteredEvents = computed(() => {
    let result = events.value

    // Filter by current variant if set
    if (currentVariantId.value) {
      result = result.filter(e => e.variantId === currentVariantId.value)
    }

    // Filter by type
    if (filterType.value) {
      result = result.filter(e => e.type.includes(filterType.value))
    }

    return result
  })

  const eventTypes = computed(() => {
    const targetEvents = currentVariantId.value
      ? events.value.filter(e => e.variantId === currentVariantId.value)
      : events.value
    const types = new Set(targetEvents.map(e => e.type))
    return Array.from(types).sort()
  })

  const eventCounts = computed(() => {
    const targetEvents = currentVariantId.value
      ? events.value.filter(e => e.variantId === currentVariantId.value)
      : events.value
    const counts: Record<string, number> = {}
    for (const event of targetEvents) {
      counts[event.type] = (counts[event.type] || 0) + 1
    }
    return counts
  })

  const addEvent = (eventData: Omit<CapturedEvent, 'id' | 'timestamp'> | CapturedEvent) => {
    if (isPaused.value) return

    const event: CapturedEvent = {
      id: 'id' in eventData ? eventData.id : `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`,
      timestamp: 'timestamp' in eventData ? eventData.timestamp : Date.now(),
      type: eventData.type,
      target: eventData.target,
      payload: eventData.payload,
      variantId: eventData.variantId,
      rawEvent: eventData.rawEvent,
    }

    // Trigger reactivity for shallowRef
    const newEvents = [...events.value, event]
    if (newEvents.length > maxEvents) {
      events.value = newEvents.slice(-maxEvents)
    } else {
      events.value = newEvents
    }

    // Store per-variant
    const variantId = event.variantId
    if (!variantEvents.value.has(variantId)) {
      variantEvents.value.set(variantId, [])
    }
    const variantList = variantEvents.value.get(variantId)!
    variantList.push(event)
    if (variantList.length > maxEvents) {
      variantEvents.value.set(variantId, variantList.slice(-maxEvents))
    }
  }

  const clearEvents = () => {
    if (currentVariantId.value) {
      // Clear only current variant events
      clearVariantEvents(currentVariantId.value)
    } else {
      // Clear all events
      events.value = []
      variantEvents.value.clear()
    }
  }

  const clearVariantEvents = (variantId: string) => {
    variantEvents.value.delete(variantId)
    events.value = events.value.filter(e => e.variantId !== variantId)
  }

  const getVariantEvents = (variantId: string): CapturedEvent[] => {
    return variantEvents.value.get(variantId) || []
  }

  const setCurrentVariant = (variantId: string) => {
    currentVariantId.value = variantId
  }

  const togglePause = () => {
    isPaused.value = !isPaused.value
  }

  const setFilter = (type: string) => {
    filterType.value = type
  }

  return {
    events,
    filteredEvents,
    eventTypes,
    eventCounts,
    filterType,
    isPaused,
    currentVariantId,
    addEvent,
    clearEvents,
    clearVariantEvents,
    getVariantEvents,
    setCurrentVariant,
    togglePause,
    setFilter,
  }
}
