import { onUnmounted, type Ref } from 'vue'

export interface MuseaMessage {
  type: string
  payload: unknown
}

export function sendMessage(iframe: HTMLIFrameElement, type: string, payload: unknown = {}): void {
  iframe.contentWindow?.postMessage({ type, payload }, '*')
}

export function sendMessageToAll(iframes: Ref<HTMLIFrameElement[]>, type: string, payload: unknown = {}): void {
  for (const iframe of iframes.value) {
    sendMessage(iframe, type, payload)
  }
}

export function useMessageListener(
  type: string,
  callback: (payload: unknown) => void,
): void {
  const handler = (event: MessageEvent) => {
    if (event.origin !== window.location.origin) return
    const data = event.data as MuseaMessage | undefined
    if (!data?.type?.startsWith('musea:')) return
    if (data.type === type) {
      callback(data.payload)
    }
  }

  window.addEventListener('message', handler)
  onUnmounted(() => {
    window.removeEventListener('message', handler)
  })
}
