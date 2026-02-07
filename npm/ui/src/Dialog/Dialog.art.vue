<script setup>
import { Dialog } from '../index'
</script>

<art title="Dialog" component="./DialogRoot.vue" category="Overlays" status="ready" tags="dialog,modal,popup">
  <variant name="Default" skip-vrt>
    <Dialog.Root>
      <Dialog.Trigger>Open Dialog</Dialog.Trigger>
      <Dialog.Portal disabled>
        <Dialog.Overlay />
        <Dialog.Content>
          <Dialog.Title>Dialog Title</Dialog.Title>
          <Dialog.Description>Dialog description text.</Dialog.Description>
          <Dialog.Close>Close</Dialog.Close>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  </variant>

  <variant name="Default Open" skip-vrt>
    <Dialog.Root :default-value="true">
      <Dialog.Trigger>Open Dialog</Dialog.Trigger>
      <Dialog.Portal disabled>
        <Dialog.Overlay />
        <Dialog.Content>
          <Dialog.Title>Dialog Title</Dialog.Title>
          <Dialog.Description>Dialog description text.</Dialog.Description>
          <Dialog.Close>Close</Dialog.Close>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  </variant>
</art>
