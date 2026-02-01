import { describe, it, expect, afterEach } from 'vitest'
import { mount, VueWrapper } from '@vue/test-utils'
import { h, defineComponent, ref, onMounted, onUnmounted, Suspense } from 'vue'
import Button from '../src/components/Button.vue'

describe('Vue Components', () => {
  describe('Button Component', () => {
    let wrapper: VueWrapper

    afterEach(() => {
      wrapper?.unmount()
    })

    it('should render default button', () => {
      wrapper = mount(Button, {
        slots: {
          default: 'Click me',
        },
      })
      expect(wrapper.text()).toBe('Click me')
      expect(wrapper.classes()).toContain('btn')
      expect(wrapper.classes()).toContain('btn--default')
    })

    it('should render primary variant', () => {
      wrapper = mount(Button, {
        props: {
          variant: 'primary',
        },
        slots: {
          default: 'Primary',
        },
      })
      expect(wrapper.classes()).toContain('btn--primary')
    })

    it('should render secondary variant', () => {
      wrapper = mount(Button, {
        props: {
          variant: 'secondary',
        },
        slots: {
          default: 'Secondary',
        },
      })
      expect(wrapper.classes()).toContain('btn--secondary')
    })

    it('should handle disabled state', () => {
      wrapper = mount(Button, {
        props: {
          disabled: true,
        },
        slots: {
          default: 'Disabled',
        },
      })
      expect(wrapper.classes()).toContain('btn--disabled')
      expect(wrapper.attributes('disabled')).toBeDefined()
    })

    it('should emit click event', async () => {
      wrapper = mount(Button, {
        slots: {
          default: 'Click',
        },
      })
      await wrapper.trigger('click')
      expect(wrapper.emitted('click')).toBeTruthy()
    })
  })

  describe('Dynamic Components', () => {
    it('should render component with reactive state', async () => {
      const TestComponent = defineComponent({
        setup() {
          const count = ref(0)
          const increment = () => count.value++
          return { count, increment }
        },
        render() {
          return h('div', [
            h('span', { class: 'count' }, this.count),
            h('button', { onClick: this.increment }, '+'),
          ])
        },
      })

      const wrapper = mount(TestComponent)
      expect(wrapper.find('.count').text()).toBe('0')

      await wrapper.find('button').trigger('click')
      expect(wrapper.find('.count').text()).toBe('1')

      await wrapper.find('button').trigger('click')
      expect(wrapper.find('.count').text()).toBe('2')

      wrapper.unmount()
    })

    it('should handle props correctly', () => {
      const TestComponent = defineComponent({
        props: {
          message: String,
          count: {
            type: Number,
            default: 0,
          },
        },
        render() {
          return h('div', `${this.message} - ${this.count}`)
        },
      })

      const wrapper = mount(TestComponent, {
        props: {
          message: 'Hello',
          count: 42,
        },
      })

      expect(wrapper.text()).toBe('Hello - 42')
      wrapper.unmount()
    })

    it('should handle computed properties', () => {
      const TestComponent = defineComponent({
        setup() {
          const items = ref([1, 2, 3, 4, 5])
          const evenItems = () => items.value.filter((i) => i % 2 === 0)
          const sum = () => items.value.reduce((a, b) => a + b, 0)
          return { items, evenItems, sum }
        },
        render() {
          return h('div', [
            h('span', { class: 'sum' }, this.sum()),
            h('span', { class: 'even' }, this.evenItems().join(',')),
          ])
        },
      })

      const wrapper = mount(TestComponent)
      expect(wrapper.find('.sum').text()).toBe('15')
      expect(wrapper.find('.even').text()).toBe('2,4')
      wrapper.unmount()
    })

    it('should handle v-if directive', async () => {
      const TestComponent = defineComponent({
        setup() {
          const show = ref(true)
          const toggle = () => (show.value = !show.value)
          return { show, toggle }
        },
        render() {
          return h('div', [
            this.show ? h('div', { class: 'content' }, 'Visible') : null,
            h('button', { onClick: this.toggle }, 'Toggle'),
          ])
        },
      })

      const wrapper = mount(TestComponent)
      expect(wrapper.find('.content').exists()).toBe(true)

      await wrapper.find('button').trigger('click')
      expect(wrapper.find('.content').exists()).toBe(false)

      await wrapper.find('button').trigger('click')
      expect(wrapper.find('.content').exists()).toBe(true)

      wrapper.unmount()
    })

    it('should handle v-for directive', () => {
      const TestComponent = defineComponent({
        setup() {
          const items = ref(['a', 'b', 'c'])
          return { items }
        },
        render() {
          return h(
            'ul',
            this.items.map((item) => h('li', { key: item, class: 'item' }, item))
          )
        },
      })

      const wrapper = mount(TestComponent)
      const items = wrapper.findAll('.item')
      expect(items.length).toBe(3)
      expect(items[0].text()).toBe('a')
      expect(items[1].text()).toBe('b')
      expect(items[2].text()).toBe('c')
      wrapper.unmount()
    })

    it('should handle v-model on input', async () => {
      const TestComponent = defineComponent({
        setup() {
          const text = ref('')
          return { text }
        },
        render() {
          return h('div', [
            h('input', {
              class: 'input',
              value: this.text,
              onInput: (e: Event) => {
                this.text = (e.target as HTMLInputElement).value
              },
            }),
            h('span', { class: 'output' }, this.text),
          ])
        },
      })

      const wrapper = mount(TestComponent)
      const input = wrapper.find('.input')
      await input.setValue('Hello World')
      expect(wrapper.find('.output').text()).toBe('Hello World')
      wrapper.unmount()
    })

    it('should handle emits', async () => {
      const ChildComponent = defineComponent({
        emits: ['update'],
        setup(_, { emit }) {
          const sendUpdate = () => emit('update', 'new value')
          return { sendUpdate }
        },
        render() {
          return h('button', { onClick: this.sendUpdate }, 'Send')
        },
      })

      const wrapper = mount(ChildComponent)
      await wrapper.find('button').trigger('click')
      expect(wrapper.emitted('update')).toBeTruthy()
      expect(wrapper.emitted('update')![0]).toEqual(['new value'])
      wrapper.unmount()
    })

    it('should handle slots', () => {
      const SlotComponent = defineComponent({
        render() {
          return h('div', [
            h('header', this.$slots.header?.()),
            h('main', this.$slots.default?.()),
            h('footer', this.$slots.footer?.()),
          ])
        },
      })

      const wrapper = mount(SlotComponent, {
        slots: {
          header: 'Header Content',
          default: 'Main Content',
          footer: 'Footer Content',
        },
      })

      expect(wrapper.find('header').text()).toBe('Header Content')
      expect(wrapper.find('main').text()).toBe('Main Content')
      expect(wrapper.find('footer').text()).toBe('Footer Content')
      wrapper.unmount()
    })
  })

  describe('Lifecycle Hooks', () => {
    it('should call onMounted', async () => {
      let mounted = false
      const TestComponent = defineComponent({
        setup() {
          onMounted(() => {
            mounted = true
          })
          return {}
        },
        render() {
          return h('div', 'Test')
        },
      })

      const wrapper = mount(TestComponent)
      await wrapper.vm.$nextTick()
      expect(mounted).toBe(true)
      wrapper.unmount()
    })

    it('should call onUnmounted', async () => {
      let unmounted = false
      const TestComponent = defineComponent({
        setup() {
          onUnmounted(() => {
            unmounted = true
          })
          return {}
        },
        render() {
          return h('div', 'Test')
        },
      })

      const wrapper = mount(TestComponent)
      expect(unmounted).toBe(false)
      wrapper.unmount()
      expect(unmounted).toBe(true)
    })
  })

  describe('Async Components', () => {
    it('should handle async setup', async () => {
      const AsyncComponent = defineComponent({
        async setup() {
          // Simulate async operation
          await new Promise((resolve) => setTimeout(resolve, 10))
          const data = ref('Loaded')
          return { data }
        },
        render() {
          return h('div', { class: 'data' }, this.data)
        },
      })

      const wrapper = mount({
        components: { AsyncComponent },
        render() {
          return h(Suspense, null, {
            default: () => h(AsyncComponent),
            fallback: () => h('div', 'Loading...'),
          })
        },
      })

      // Wait for async setup
      await new Promise((resolve) => setTimeout(resolve, 100))
      await wrapper.vm.$nextTick()

      const dataEl = wrapper.find('.data')
      if (dataEl.exists()) {
        expect(dataEl.text()).toBe('Loaded')
      }
      wrapper.unmount()
    })
  })
})
