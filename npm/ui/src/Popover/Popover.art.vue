<script setup>
import { Popover } from '../index'
</script>

<art title="Popover" component="./PopoverRoot.vue" category="Overlays" status="ready" tags="popover,popup,dropdown">
  <variant name="Default" skip-vrt>
    <Popover.Root>
      <Popover.Trigger>Open Popover</Popover.Trigger>
      <Popover.Portal disabled>
        <Popover.Content>
          <Popover.Arrow />
          <p>Popover content goes here.</p>
          <Popover.Close>Close</Popover.Close>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  </variant>

  <variant name="Default Open" skip-vrt>
    <Popover.Root :default-value="true">
      <Popover.Trigger>Open Popover</Popover.Trigger>
      <Popover.Portal disabled>
        <Popover.Content>
          <Popover.Arrow />
          <p>Popover content goes here.</p>
          <Popover.Close>Close</Popover.Close>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  </variant>
</art>
