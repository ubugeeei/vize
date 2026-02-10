<script setup>
import { PopoverRoot, PopoverTrigger, PopoverPortal, PopoverContent, PopoverClose, PopoverArrow } from './index'
</script>

<art title="Popover" component="./PopoverRoot.vue" category="Overlays" status="ready" tags="popover,popup,dropdown">
  <variant name="Default" skip-vrt>
    <PopoverRoot>
      <PopoverTrigger>Open Popover</PopoverTrigger>
      <PopoverPortal disabled>
        <PopoverContent>
          <PopoverArrow />
          <p>Popover content goes here.</p>
          <PopoverClose>Close</PopoverClose>
        </PopoverContent>
      </PopoverPortal>
    </PopoverRoot>
  </variant>

  <variant name="Default Open" skip-vrt>
    <PopoverRoot :default-value="true">
      <PopoverTrigger>Open Popover</PopoverTrigger>
      <PopoverPortal disabled>
        <PopoverContent>
          <PopoverArrow />
          <p>Popover content goes here.</p>
          <PopoverClose>Close</PopoverClose>
        </PopoverContent>
      </PopoverPortal>
    </PopoverRoot>
  </variant>
</art>
