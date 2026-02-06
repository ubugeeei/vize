import { ref } from 'vue'
import { useMessageListener } from './usePostMessage'

export interface ActionEvent {
  name: string
  target?: string
  value?: unknown
  args?: unknown
  timestamp: number
  source: 'dom' | 'vue'
}

const events = ref<ActionEvent[]>([])

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

  return {
    events,
    init,
    clear,
  }
}
