<script setup>
import { Checkbox } from '../index'
</script>

<art title="Checkbox" component="./CheckboxRoot.vue" category="Forms" status="ready" tags="checkbox,input,form">
  <variant name="Default" default>
    <Checkbox.Root>
      <Checkbox.Indicator>✓</Checkbox.Indicator>
    </Checkbox.Root>
  </variant>

  <variant name="Checked">
    <Checkbox.Root :default-value="true">
      <Checkbox.Indicator>✓</Checkbox.Indicator>
    </Checkbox.Root>
  </variant>

  <variant name="Indeterminate">
    <Checkbox.Root :default-value="'indeterminate'">
      <Checkbox.Indicator>−</Checkbox.Indicator>
    </Checkbox.Root>
  </variant>

  <variant name="Disabled">
    <Checkbox.Root disabled>
      <Checkbox.Indicator>✓</Checkbox.Indicator>
    </Checkbox.Root>
  </variant>
</art>
