<script setup>
import { Toggle } from '../index'
</script>

<art title="Toggle" component="./ToggleRoot.vue" category="Actions" status="ready" tags="toggle,switch,press">
  <variant name="Default" default>
    <Toggle.Root v-slot="{ pressed }">
      {{ pressed ? 'On' : 'Off' }}
    </Toggle.Root>
  </variant>

  <variant name="Pressed">
    <Toggle.Root :default-value="true" v-slot="{ pressed }">
      {{ pressed ? 'On' : 'Off' }}
    </Toggle.Root>
  </variant>

  <variant name="Disabled">
    <Toggle.Root disabled v-slot="{ pressed }">
      {{ pressed ? 'On' : 'Off' }}
    </Toggle.Root>
  </variant>
</art>
