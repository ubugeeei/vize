<script setup>
import { SwitchRoot, SwitchThumb } from './index'
</script>

<art title="Switch" component="./SwitchRoot.vue" category="Forms" status="ready" tags="switch,toggle,form">
  <variant name="Default" default>
    <SwitchRoot>
      <SwitchThumb />
    </SwitchRoot>
  </variant>

  <variant name="Checked">
    <SwitchRoot :default-value="true">
      <SwitchThumb />
    </SwitchRoot>
  </variant>

  <variant name="Disabled">
    <SwitchRoot disabled>
      <SwitchThumb />
    </SwitchRoot>
  </variant>
</art>
