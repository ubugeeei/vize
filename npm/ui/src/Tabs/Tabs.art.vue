<script setup>
import { Tabs } from '../index'
</script>

<art title="Tabs" component="./TabsRoot.vue" category="Navigation" status="ready" tags="tabs,tablist,tabpanel">
  <variant name="Default" default>
    <Tabs.Root default-value="tab1">
      <Tabs.List>
        <Tabs.Trigger value="tab1">Tab 1</Tabs.Trigger>
        <Tabs.Trigger value="tab2">Tab 2</Tabs.Trigger>
        <Tabs.Trigger value="tab3">Tab 3</Tabs.Trigger>
      </Tabs.List>
      <Tabs.Content value="tab1">Content for tab 1</Tabs.Content>
      <Tabs.Content value="tab2">Content for tab 2</Tabs.Content>
      <Tabs.Content value="tab3">Content for tab 3</Tabs.Content>
    </Tabs.Root>
  </variant>

  <variant name="Vertical">
    <Tabs.Root default-value="tab1" orientation="vertical">
      <Tabs.List>
        <Tabs.Trigger value="tab1">Tab 1</Tabs.Trigger>
        <Tabs.Trigger value="tab2">Tab 2</Tabs.Trigger>
        <Tabs.Trigger value="tab3">Tab 3</Tabs.Trigger>
      </Tabs.List>
      <Tabs.Content value="tab1">Content for tab 1</Tabs.Content>
      <Tabs.Content value="tab2">Content for tab 2</Tabs.Content>
      <Tabs.Content value="tab3">Content for tab 3</Tabs.Content>
    </Tabs.Root>
  </variant>

  <variant name="Disabled Tab">
    <Tabs.Root default-value="tab1">
      <Tabs.List>
        <Tabs.Trigger value="tab1">Tab 1</Tabs.Trigger>
        <Tabs.Trigger value="tab2" disabled>Tab 2</Tabs.Trigger>
        <Tabs.Trigger value="tab3">Tab 3</Tabs.Trigger>
      </Tabs.List>
      <Tabs.Content value="tab1">Content for tab 1</Tabs.Content>
      <Tabs.Content value="tab2">Content for tab 2</Tabs.Content>
      <Tabs.Content value="tab3">Content for tab 3</Tabs.Content>
    </Tabs.Root>
  </variant>
</art>
