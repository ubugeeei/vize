<script setup>
import { Tooltip } from '../index'
</script>

<art title="Tooltip" component="./TooltipRoot.vue" category="Overlays" status="ready" tags="tooltip,hint,popup">
  <variant name="Default" skip-vrt>
    <Tooltip.Provider>
      <Tooltip.Root>
        <Tooltip.Trigger>Hover me</Tooltip.Trigger>
        <Tooltip.Portal disabled>
          <Tooltip.Content>
            Tooltip text
            <Tooltip.Arrow />
          </Tooltip.Content>
        </Tooltip.Portal>
      </Tooltip.Root>
    </Tooltip.Provider>
  </variant>
</art>
