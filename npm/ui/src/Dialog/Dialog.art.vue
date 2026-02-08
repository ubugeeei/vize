<script setup>
import { DialogRoot, DialogTrigger, DialogPortal, DialogOverlay, DialogContent, DialogTitle, DialogDescription, DialogClose } from './index'
</script>

<art title="Dialog" component="./DialogRoot.vue" category="Overlays" status="ready" tags="dialog,modal,popup">
  <variant name="Default" skip-vrt>
    <DialogRoot>
      <DialogTrigger>Open Dialog</DialogTrigger>
      <DialogPortal disabled>
        <DialogOverlay />
        <DialogContent>
          <DialogTitle>Dialog Title</DialogTitle>
          <DialogDescription>Dialog description text.</DialogDescription>
          <DialogClose>Close</DialogClose>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>
  </variant>

  <variant name="Default Open" skip-vrt>
    <DialogRoot :default-value="true">
      <DialogTrigger>Open Dialog</DialogTrigger>
      <DialogPortal disabled>
        <DialogOverlay />
        <DialogContent>
          <DialogTitle>Dialog Title</DialogTitle>
          <DialogDescription>Dialog description text.</DialogDescription>
          <DialogClose>Close</DialogClose>
        </DialogContent>
      </DialogPortal>
    </DialogRoot>
  </variant>
</art>
