<script setup>
import { FileUploader } from '../index'
</script>

<art title="FileUploader" component="./FileUploaderRoot.vue" category="Forms" status="ready" tags="file,upload,drag-drop">
  <variant name="Default" default>
    <FileUploader.Root>
      <FileUploader.Dropzone>
        <FileUploader.Trigger>Choose files</FileUploader.Trigger>
        or drag &amp; drop
      </FileUploader.Dropzone>
      <FileUploader.List v-slot="{ files }">
        <FileUploader.Item v-for="file in files" :key="file.name" :file="file">
          {{ file.name }}
          <FileUploader.ItemDelete />
        </FileUploader.Item>
      </FileUploader.List>
    </FileUploader.Root>
  </variant>

  <variant name="With Accept">
    <FileUploader.Root accept="image/*">
      <FileUploader.Dropzone>
        <FileUploader.Trigger>Choose files</FileUploader.Trigger>
        or drag &amp; drop
      </FileUploader.Dropzone>
      <FileUploader.List v-slot="{ files }">
        <FileUploader.Item v-for="file in files" :key="file.name" :file="file">
          {{ file.name }}
          <FileUploader.ItemDelete />
        </FileUploader.Item>
      </FileUploader.List>
    </FileUploader.Root>
  </variant>

  <variant name="Multiple">
    <FileUploader.Root multiple>
      <FileUploader.Dropzone>
        <FileUploader.Trigger>Choose files</FileUploader.Trigger>
        or drag &amp; drop
      </FileUploader.Dropzone>
      <FileUploader.List v-slot="{ files }">
        <FileUploader.Item v-for="file in files" :key="file.name" :file="file">
          {{ file.name }}
          <FileUploader.ItemDelete />
        </FileUploader.Item>
      </FileUploader.List>
    </FileUploader.Root>
  </variant>

  <variant name="Disabled">
    <FileUploader.Root disabled>
      <FileUploader.Dropzone>
        <FileUploader.Trigger>Choose files</FileUploader.Trigger>
        or drag &amp; drop
      </FileUploader.Dropzone>
      <FileUploader.List v-slot="{ files }">
        <FileUploader.Item v-for="file in files" :key="file.name" :file="file">
          {{ file.name }}
          <FileUploader.ItemDelete />
        </FileUploader.Item>
      </FileUploader.List>
    </FileUploader.Root>
  </variant>
</art>
