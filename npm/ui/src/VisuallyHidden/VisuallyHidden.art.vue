<script setup>
import { VisuallyHidden } from './index'
</script>

<art title="VisuallyHidden" component="./VisuallyHidden.vue" category="Utilities" status="ready" tags="a11y,screen-reader,hidden">
  <variant name="Default" default>
    <button>
      Save
      <VisuallyHidden>changes to document</VisuallyHidden>
    </button>
    <p><em>The text "changes to document" is visually hidden but accessible to screen readers.</em></p>
  </variant>
</art>
