<script setup>
import { TooltipProvider, TooltipRoot, TooltipTrigger, TooltipPortal, TooltipContent, TooltipArrow } from './index'
</script>

<art title="Tooltip" component="./TooltipRoot.vue" category="Overlays" status="ready" tags="tooltip,hint,popup">
  <variant name="Default" skip-vrt>
    <TooltipProvider>
      <TooltipRoot>
        <TooltipTrigger>Hover me</TooltipTrigger>
        <TooltipPortal disabled>
          <TooltipContent>
            Tooltip text
            <TooltipArrow />
          </TooltipContent>
        </TooltipPortal>
      </TooltipRoot>
    </TooltipProvider>
  </variant>
</art>
