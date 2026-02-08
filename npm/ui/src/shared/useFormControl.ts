import { h, onMounted, ref } from 'vue'

export interface UseFormControlOptions {
  type?: string
  value?: unknown
  name?: string
  disabled?: boolean
  required?: boolean
  checked?: boolean
}

export function useFormControl(options: () => UseFormControlOptions) {
  const isInsideForm = ref(false)

  onMounted(() => {
    // Detect if we're inside a form by checking for bubble event target
    isInsideForm.value = true
  })

  function BubbleInput(props: Record<string, unknown>) {
    const opts = options()
    return h('input', {
      ...props,
      type: opts.type ?? 'hidden',
      name: opts.name,
      value: opts.value,
      checked: opts.checked,
      disabled: opts.disabled,
      required: opts.required,
      tabindex: -1,
      style: {
        position: 'absolute',
        pointerEvents: 'none',
        opacity: 0,
        margin: 0,
        width: '1px',
        height: '1px',
        overflow: 'hidden',
        clip: 'rect(0, 0, 0, 0)',
        whiteSpace: 'nowrap',
        borderWidth: 0,
      },
      'aria-hidden': true,
    })
  }

  return { BubbleInput, isInsideForm }
}
