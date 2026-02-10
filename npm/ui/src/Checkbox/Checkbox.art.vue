<script setup>
import { CheckboxRoot, CheckboxIndicator } from './index'
</script>

<art title="Checkbox" component="./CheckboxRoot.vue" category="Forms" status="ready" tags="checkbox,input,form">
  <variant name="Default" default>
    <CheckboxRoot aria-label="Accept terms">
      <CheckboxIndicator>✓</CheckboxIndicator>
    </CheckboxRoot>
  </variant>

  <variant name="Checked">
    <CheckboxRoot :default-value="true" aria-label="Accept terms">
      <CheckboxIndicator>✓</CheckboxIndicator>
    </CheckboxRoot>
  </variant>

  <variant name="Indeterminate">
    <CheckboxRoot :default-value="'indeterminate'" aria-label="Select all">
      <CheckboxIndicator>−</CheckboxIndicator>
    </CheckboxRoot>
  </variant>

  <variant name="Disabled">
    <CheckboxRoot disabled aria-label="Accept terms">
      <CheckboxIndicator>✓</CheckboxIndicator>
    </CheckboxRoot>
  </variant>
</art>
