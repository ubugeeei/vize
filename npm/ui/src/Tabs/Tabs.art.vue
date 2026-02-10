<script setup>
import { TabsRoot, TabsList, TabsTrigger, TabsContent } from './index'
</script>

<art title="Tabs" component="./TabsRoot.vue" category="Navigation" status="ready" tags="tabs,tablist,tabpanel">
  <variant name="Default" default>
    <TabsRoot default-value="tab1">
      <TabsList>
        <TabsTrigger value="tab1">Tab 1</TabsTrigger>
        <TabsTrigger value="tab2">Tab 2</TabsTrigger>
        <TabsTrigger value="tab3">Tab 3</TabsTrigger>
      </TabsList>
      <TabsContent value="tab1">Content for tab 1</TabsContent>
      <TabsContent value="tab2">Content for tab 2</TabsContent>
      <TabsContent value="tab3">Content for tab 3</TabsContent>
    </TabsRoot>
  </variant>

  <variant name="Vertical">
    <TabsRoot default-value="tab1" orientation="vertical">
      <TabsList>
        <TabsTrigger value="tab1">Tab 1</TabsTrigger>
        <TabsTrigger value="tab2">Tab 2</TabsTrigger>
        <TabsTrigger value="tab3">Tab 3</TabsTrigger>
      </TabsList>
      <TabsContent value="tab1">Content for tab 1</TabsContent>
      <TabsContent value="tab2">Content for tab 2</TabsContent>
      <TabsContent value="tab3">Content for tab 3</TabsContent>
    </TabsRoot>
  </variant>

  <variant name="Disabled Tab">
    <TabsRoot default-value="tab1">
      <TabsList>
        <TabsTrigger value="tab1">Tab 1</TabsTrigger>
        <TabsTrigger value="tab2" disabled>Tab 2</TabsTrigger>
        <TabsTrigger value="tab3">Tab 3</TabsTrigger>
      </TabsList>
      <TabsContent value="tab1">Content for tab 1</TabsContent>
      <TabsContent value="tab2">Content for tab 2</TabsContent>
      <TabsContent value="tab3">Content for tab 3</TabsContent>
    </TabsRoot>
  </variant>
</art>
