<script setup>
import { VirtualScroll } from './index'

const items = Array.from({ length: 1000 }, (_, i) => ({ id: i, label: `Item ${i}` }))
</script>

<art title="VirtualScroll" component="./VirtualScroll.vue" category="Data Display" status="ready" tags="virtual,scroll,list,performance">
  <variant name="Default" default>
    <VirtualScroll :items="items" :item-height="40" v-slot="{ virtualItems, totalSize, containerProps }" style="height:300px;overflow:auto">
      <div v-bind="containerProps">
        <div v-for="item in virtualItems" :key="item.index" :style="{ position: 'absolute', top: item.start + 'px', height: item.size + 'px', width: '100%' }">
          {{ item.item.label }}
        </div>
      </div>
    </VirtualScroll>
  </variant>
</art>
