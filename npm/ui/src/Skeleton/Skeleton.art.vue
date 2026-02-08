<script setup>
import { Skeleton } from './index'
</script>

<art title="Skeleton" component="./Skeleton.vue" category="Feedback" status="ready" tags="skeleton,loading,placeholder">
  <variant name="Default" default>
    <Skeleton />
  </variant>

  <variant name="Circle">
    <Skeleton circle :width="48" />
  </variant>

  <variant name="Custom Size">
    <Skeleton :width="200" :height="16" />
  </variant>

  <variant name="With Content">
    <Skeleton :loading="false">
      <p>Content has loaded</p>
    </Skeleton>
  </variant>
</art>
