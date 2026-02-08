<script setup>
import { AccordionRoot, AccordionItem, AccordionHeader, AccordionTrigger, AccordionContent } from './index'
</script>

<art title="Accordion" component="./AccordionRoot.vue" category="Disclosure" status="ready" tags="accordion,expand,collapse,faq">
  <variant name="Single" default>
    <AccordionRoot type="single">
      <AccordionItem value="item-1">
        <AccordionHeader>
          <AccordionTrigger>Item 1</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 1</AccordionContent>
      </AccordionItem>
      <AccordionItem value="item-2">
        <AccordionHeader>
          <AccordionTrigger>Item 2</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 2</AccordionContent>
      </AccordionItem>
      <AccordionItem value="item-3">
        <AccordionHeader>
          <AccordionTrigger>Item 3</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 3</AccordionContent>
      </AccordionItem>
    </AccordionRoot>
  </variant>

  <variant name="Multiple">
    <AccordionRoot type="multiple">
      <AccordionItem value="item-1">
        <AccordionHeader>
          <AccordionTrigger>Item 1</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 1</AccordionContent>
      </AccordionItem>
      <AccordionItem value="item-2">
        <AccordionHeader>
          <AccordionTrigger>Item 2</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 2</AccordionContent>
      </AccordionItem>
      <AccordionItem value="item-3">
        <AccordionHeader>
          <AccordionTrigger>Item 3</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 3</AccordionContent>
      </AccordionItem>
    </AccordionRoot>
  </variant>

  <variant name="Default Value">
    <AccordionRoot type="single" default-value="item-2">
      <AccordionItem value="item-1">
        <AccordionHeader>
          <AccordionTrigger>Item 1</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 1</AccordionContent>
      </AccordionItem>
      <AccordionItem value="item-2">
        <AccordionHeader>
          <AccordionTrigger>Item 2</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 2</AccordionContent>
      </AccordionItem>
      <AccordionItem value="item-3">
        <AccordionHeader>
          <AccordionTrigger>Item 3</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 3</AccordionContent>
      </AccordionItem>
    </AccordionRoot>
  </variant>

  <variant name="Collapsible">
    <AccordionRoot type="single" collapsible>
      <AccordionItem value="item-1">
        <AccordionHeader>
          <AccordionTrigger>Item 1</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 1</AccordionContent>
      </AccordionItem>
      <AccordionItem value="item-2">
        <AccordionHeader>
          <AccordionTrigger>Item 2</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 2</AccordionContent>
      </AccordionItem>
      <AccordionItem value="item-3">
        <AccordionHeader>
          <AccordionTrigger>Item 3</AccordionTrigger>
        </AccordionHeader>
        <AccordionContent>Content for item 3</AccordionContent>
      </AccordionItem>
    </AccordionRoot>
  </variant>
</art>
