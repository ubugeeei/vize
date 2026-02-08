<script setup>
import { CollapsibleRoot, CollapsibleTrigger, CollapsibleContent } from './index'
</script>

<art title="Collapsible" component="./CollapsibleRoot.vue" category="Disclosure" status="ready" tags="collapsible,expand,collapse">
  <variant name="Default" default>
    <CollapsibleRoot>
      <CollapsibleTrigger>Toggle</CollapsibleTrigger>
      <CollapsibleContent>Collapsible content</CollapsibleContent>
    </CollapsibleRoot>
  </variant>

  <variant name="Open">
    <CollapsibleRoot default-open>
      <CollapsibleTrigger>Toggle</CollapsibleTrigger>
      <CollapsibleContent>Collapsible content</CollapsibleContent>
    </CollapsibleRoot>
  </variant>

  <variant name="Disabled">
    <CollapsibleRoot disabled>
      <CollapsibleTrigger>Toggle</CollapsibleTrigger>
      <CollapsibleContent>Collapsible content</CollapsibleContent>
    </CollapsibleRoot>
  </variant>
</art>
