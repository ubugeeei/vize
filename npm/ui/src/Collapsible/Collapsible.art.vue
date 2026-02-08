<script setup>
import { Collapsible } from '../index'
</script>

<art title="Collapsible" component="./CollapsibleRoot.vue" category="Disclosure" status="ready" tags="collapsible,expand,collapse">
  <variant name="Default" default>
    <Collapsible.Root>
      <Collapsible.Trigger>Toggle</Collapsible.Trigger>
      <Collapsible.Content>Collapsible content</Collapsible.Content>
    </Collapsible.Root>
  </variant>

  <variant name="Open">
    <Collapsible.Root default-open>
      <Collapsible.Trigger>Toggle</Collapsible.Trigger>
      <Collapsible.Content>Collapsible content</Collapsible.Content>
    </Collapsible.Root>
  </variant>

  <variant name="Disabled">
    <Collapsible.Root disabled>
      <Collapsible.Trigger>Toggle</Collapsible.Trigger>
      <Collapsible.Content>Collapsible content</Collapsible.Content>
    </Collapsible.Root>
  </variant>
</art>
