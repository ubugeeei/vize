import { ref, onMounted, onUnmounted, watch } from 'vue'

export interface ResizableOptions {
  direction: 'horizontal' | 'vertical'
  minSize: number
  maxSize: number
  storageKey?: string
  defaultSize: number
}

export function useResizable(options: ResizableOptions) {
  const {
    direction,
    minSize,
    maxSize,
    storageKey,
    defaultSize,
  } = options

  const size = ref(defaultSize)
  const isResizing = ref(false)
  const startPos = ref(0)
  const startSize = ref(0)

  // Load from localStorage
  onMounted(() => {
    if (storageKey) {
      const saved = localStorage.getItem(storageKey)
      if (saved) {
        const parsed = parseInt(saved, 10)
        if (!isNaN(parsed) && parsed >= minSize && parsed <= maxSize) {
          size.value = parsed
        }
      }
    }
  })

  // Save to localStorage
  watch(size, (newSize) => {
    if (storageKey) {
      localStorage.setItem(storageKey, String(newSize))
    }
  })

  const onMouseDown = (e: MouseEvent) => {
    e.preventDefault()
    isResizing.value = true
    startPos.value = direction === 'horizontal' ? e.clientX : e.clientY
    startSize.value = size.value

    document.addEventListener('mousemove', onMouseMove)
    document.addEventListener('mouseup', onMouseUp)
    document.body.style.cursor = direction === 'horizontal' ? 'col-resize' : 'row-resize'
    document.body.style.userSelect = 'none'
  }

  const onMouseMove = (e: MouseEvent) => {
    if (!isResizing.value) return

    const currentPos = direction === 'horizontal' ? e.clientX : e.clientY
    const delta = currentPos - startPos.value
    let newSize = startSize.value + delta

    // Clamp to min/max
    newSize = Math.max(minSize, Math.min(maxSize, newSize))
    size.value = newSize
  }

  const onMouseUp = () => {
    isResizing.value = false
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
    document.body.style.cursor = ''
    document.body.style.userSelect = ''
  }

  onUnmounted(() => {
    document.removeEventListener('mousemove', onMouseMove)
    document.removeEventListener('mouseup', onMouseUp)
  })

  const reset = () => {
    size.value = defaultSize
    if (storageKey) {
      localStorage.removeItem(storageKey)
    }
  }

  return {
    size,
    isResizing,
    onMouseDown,
    reset,
  }
}

// Composable for the full gallery layout with multiple resizable panels
export function useResizableLayout() {
  const sidebarWidth = useResizable({
    direction: 'horizontal',
    minSize: 180,
    maxSize: 400,
    storageKey: 'musea-sidebar-width',
    defaultSize: 240,
  })

  const propsWidth = useResizable({
    direction: 'horizontal',
    minSize: 200,
    maxSize: 500,
    storageKey: 'musea-props-width',
    defaultSize: 280,
  })

  const eventPanelHeight = useResizable({
    direction: 'vertical',
    minSize: 100,
    maxSize: 400,
    storageKey: 'musea-event-height',
    defaultSize: 200,
  })

  return {
    sidebarWidth,
    propsWidth,
    eventPanelHeight,
  }
}
