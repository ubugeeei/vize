<script setup>
import { Label } from './index'
</script>

<art title="Label" component="./Label.vue" category="Typography" status="ready" tags="label,text,form">
  <variant name="Default" default>
    <Label>Username</Label>
  </variant>

  <variant name="For Input">
    <Label for="email-input">Email address</Label>
    <input id="email-input" type="email" placeholder="you@example.com" />
  </variant>

  <variant name="Custom Element">
    <Label as="span">Rendered as span</Label>
  </variant>
</art>
