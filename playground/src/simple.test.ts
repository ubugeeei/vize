import { describe, it, expect } from 'vitest'
import { mount, shallowMount } from '@vue/test-utils'
import { h, defineComponent } from 'vue'
import Button from './components/Button.vue'

describe('Component Test', () => {
  it('should mount simple inline component', () => {
    const SimpleComp = defineComponent({
      setup() {
        return () => h('div', 'Hello')
      }
    })
    const wrapper = mount(SimpleComp)
    expect(wrapper.text()).toBe('Hello')
    wrapper.unmount()
  })

  it('should mount Button without slot', () => {
    const wrapper = mount(Button)
    expect(wrapper.find('button').exists()).toBe(true)
    wrapper.unmount()
  })

  it('should mount Button with slot', () => {
    const wrapper = mount(Button, {
      slots: { default: 'Click me' },
    })
    expect(wrapper.text()).toBe('Click me')
    wrapper.unmount()
  })
})
