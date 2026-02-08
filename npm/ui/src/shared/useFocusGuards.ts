import { onBeforeUnmount, onMounted } from 'vue'

let count = 0

export function useFocusGuards() {
  onMounted(() => {
    if (typeof document === 'undefined') return

    count++
    if (count === 1) {
      const edgeGuards = document.querySelectorAll('[data-vize-focus-guard]')
      document.querySelectorAll('[data-vize-focus-guard]').forEach((el) => el.remove())

      if (edgeGuards.length === 0) {
        const createGuard = () => {
          const guard = document.createElement('span')
          guard.setAttribute('data-vize-focus-guard', '')
          guard.tabIndex = 0
          guard.style.cssText =
            'outline:none;opacity:0;position:fixed;pointer-events:none'
          return guard
        }

        document.body.insertAdjacentElement('afterbegin', createGuard())
        document.body.insertAdjacentElement('beforeend', createGuard())
      }
    }
  })

  onBeforeUnmount(() => {
    count--
    if (count === 0) {
      document
        .querySelectorAll('[data-vize-focus-guard]')
        .forEach((el) => el.remove())
    }
  })
}
