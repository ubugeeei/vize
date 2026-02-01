import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import Button from './components/Button.vue'

describe('Slot Test', () => {
  it('should mount Button with slot', () => {
    const wrapper = mount(Button, {
      slots: { default: 'Click me' },
    })
    expect(wrapper.text()).toBe('Click me')
    wrapper.unmount()
  })

  it('should mount Button without slot', () => {
    const wrapper = mount(Button)
    expect(wrapper.find('button').exists()).toBe(true)
    wrapper.unmount()
  })
})
