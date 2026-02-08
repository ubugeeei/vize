<script setup>
import { Separator } from './index'
</script>

<art title="Separator" component="./Separator.vue" category="Layout" status="ready" tags="separator,divider,hr">
  <variant name="Horizontal" default>
    <p>Content above</p>
    <Separator />
    <p>Content below</p>
  </variant>

  <variant name="Vertical">
    <div style="display: flex; align-items: center; gap: 8px; height: 20px;">
      <span>Left</span>
      <Separator orientation="vertical" />
      <span>Right</span>
    </div>
  </variant>

  <variant name="Decorative">
    <p>Content above</p>
    <Separator decorative />
    <p>Content below</p>
  </variant>
</art>
