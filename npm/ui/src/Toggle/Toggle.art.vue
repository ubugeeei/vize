<script setup>
import { ToggleRoot } from './index'
</script>

<art title="Toggle" component="./ToggleRoot.vue" category="Actions" status="ready" tags="toggle,switch,press">
  <variant name="Default" default>
    <ToggleRoot v-slot="{ pressed }">
      {{ pressed ? 'On' : 'Off' }}
    </ToggleRoot>
  </variant>

  <variant name="Pressed">
    <ToggleRoot :default-value="true" v-slot="{ pressed }">
      {{ pressed ? 'On' : 'Off' }}
    </ToggleRoot>
  </variant>

  <variant name="Disabled">
    <ToggleRoot disabled v-slot="{ pressed }">
      {{ pressed ? 'On' : 'Off' }}
    </ToggleRoot>
  </variant>
</art>
