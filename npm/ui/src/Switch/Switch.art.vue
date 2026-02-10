<script setup>
import { SwitchRoot, SwitchThumb } from './index'
</script>

<art title="Switch" component="./SwitchRoot.vue" category="Forms" status="ready" tags="switch,toggle,form">
  <variant name="Default" default>
    <SwitchRoot aria-label="Enable notifications">
      <SwitchThumb />
    </SwitchRoot>
  </variant>

  <variant name="Checked">
    <SwitchRoot :default-value="true" aria-label="Enable notifications">
      <SwitchThumb />
    </SwitchRoot>
  </variant>

  <variant name="Disabled">
    <SwitchRoot disabled aria-label="Enable notifications">
      <SwitchThumb />
    </SwitchRoot>
  </variant>
</art>
