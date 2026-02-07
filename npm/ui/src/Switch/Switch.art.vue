<script setup>
import { Switch } from '../index'
</script>

<art title="Switch" component="./SwitchRoot.vue" category="Forms" status="ready" tags="switch,toggle,form">
  <variant name="Default" default>
    <Switch.Root>
      <Switch.Thumb />
    </Switch.Root>
  </variant>

  <variant name="Checked">
    <Switch.Root :default-value="true">
      <Switch.Thumb />
    </Switch.Root>
  </variant>

  <variant name="Disabled">
    <Switch.Root disabled>
      <Switch.Thumb />
    </Switch.Root>
  </variant>
</art>
