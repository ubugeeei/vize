<script setup>
import { CheckboxRoot, CheckboxIndicator } from './index'
</script>

<art title="Checkbox" component="./CheckboxRoot.vue" category="Forms" status="ready" tags="checkbox,input,form">
  <variant name="Default" default>
    <CheckboxRoot>
      <CheckboxIndicator>✓</CheckboxIndicator>
    </CheckboxRoot>
  </variant>

  <variant name="Checked">
    <CheckboxRoot :default-value="true">
      <CheckboxIndicator>✓</CheckboxIndicator>
    </CheckboxRoot>
  </variant>

  <variant name="Indeterminate">
    <CheckboxRoot :default-value="'indeterminate'">
      <CheckboxIndicator>−</CheckboxIndicator>
    </CheckboxRoot>
  </variant>

  <variant name="Disabled">
    <CheckboxRoot disabled>
      <CheckboxIndicator>✓</CheckboxIndicator>
    </CheckboxRoot>
  </variant>
</art>
