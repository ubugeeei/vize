import { onBeforeUnmount, watch, type Ref } from 'vue'

export function useBodyScrollLock(locked: Ref<boolean>) {
  let originalOverflow = ''
  let originalPaddingRight = ''

  function lock() {
    if (typeof document === 'undefined') return

    const scrollbarWidth =
      window.innerWidth - document.documentElement.clientWidth
    originalOverflow = document.body.style.overflow
    originalPaddingRight = document.body.style.paddingRight

    document.body.style.overflow = 'hidden'
    if (scrollbarWidth > 0) {
      document.body.style.paddingRight = `${scrollbarWidth}px`
    }
  }

  function unlock() {
    if (typeof document === 'undefined') return

    document.body.style.overflow = originalOverflow
    document.body.style.paddingRight = originalPaddingRight
  }

  watch(
    locked,
    (val) => {
      if (val) lock()
      else unlock()
    },
    { immediate: true },
  )

  onBeforeUnmount(() => {
    if (locked.value) unlock()
  })
}
