<script setup>
import { Button } from './index'
</script>

<art title="Button" component="./Button.vue" category="Actions" status="ready" tags="button,action,interactive">
  <variant name="Default" default>
    <Button>Click me</Button>
  </variant>

  <variant name="Disabled">
    <Button disabled>Disabled</Button>
  </variant>

  <variant name="As Link">
    <Button as="a" href="#">Link button</Button>
  </variant>

  <variant name="As Child">
    <Button as-child>
      <a href="#">Custom child element</a>
    </Button>
  </variant>
</art>
