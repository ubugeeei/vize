<script setup>
import { Accordion } from '../index'
</script>

<art title="Accordion" component="./AccordionRoot.vue" category="Disclosure" status="ready" tags="accordion,expand,collapse,faq">
  <variant name="Single" default>
    <Accordion.Root type="single">
      <Accordion.Item value="item-1">
        <Accordion.Header>
          <Accordion.Trigger>Item 1</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 1</Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="item-2">
        <Accordion.Header>
          <Accordion.Trigger>Item 2</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 2</Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="item-3">
        <Accordion.Header>
          <Accordion.Trigger>Item 3</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 3</Accordion.Content>
      </Accordion.Item>
    </Accordion.Root>
  </variant>

  <variant name="Multiple">
    <Accordion.Root type="multiple">
      <Accordion.Item value="item-1">
        <Accordion.Header>
          <Accordion.Trigger>Item 1</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 1</Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="item-2">
        <Accordion.Header>
          <Accordion.Trigger>Item 2</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 2</Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="item-3">
        <Accordion.Header>
          <Accordion.Trigger>Item 3</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 3</Accordion.Content>
      </Accordion.Item>
    </Accordion.Root>
  </variant>

  <variant name="Default Value">
    <Accordion.Root type="single" default-value="item-2">
      <Accordion.Item value="item-1">
        <Accordion.Header>
          <Accordion.Trigger>Item 1</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 1</Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="item-2">
        <Accordion.Header>
          <Accordion.Trigger>Item 2</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 2</Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="item-3">
        <Accordion.Header>
          <Accordion.Trigger>Item 3</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 3</Accordion.Content>
      </Accordion.Item>
    </Accordion.Root>
  </variant>

  <variant name="Collapsible">
    <Accordion.Root type="single" collapsible>
      <Accordion.Item value="item-1">
        <Accordion.Header>
          <Accordion.Trigger>Item 1</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 1</Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="item-2">
        <Accordion.Header>
          <Accordion.Trigger>Item 2</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 2</Accordion.Content>
      </Accordion.Item>
      <Accordion.Item value="item-3">
        <Accordion.Header>
          <Accordion.Trigger>Item 3</Accordion.Trigger>
        </Accordion.Header>
        <Accordion.Content>Content for item 3</Accordion.Content>
      </Accordion.Item>
    </Accordion.Root>
  </variant>
</art>
