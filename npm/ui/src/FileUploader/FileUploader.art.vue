<script setup>
import { FileUploaderRoot, FileUploaderDropzone, FileUploaderTrigger, FileUploaderList, FileUploaderItem, FileUploaderItemDelete } from './index'
</script>

<art title="FileUploader" component="./FileUploaderRoot.vue" category="Forms" status="ready" tags="file,upload,drag-drop">
  <variant name="Default" default>
    <FileUploaderRoot>
      <FileUploaderDropzone>
        <FileUploaderTrigger>Choose files</FileUploaderTrigger>
        or drag &amp; drop
      </FileUploaderDropzone>
      <FileUploaderList v-slot="{ files }">
        <FileUploaderItem v-for="file in files" :key="file.name" :file="file">
          {{ file.name }}
          <FileUploaderItemDelete />
        </FileUploaderItem>
      </FileUploaderList>
    </FileUploaderRoot>
  </variant>

  <variant name="With Accept">
    <FileUploaderRoot accept="image/*">
      <FileUploaderDropzone>
        <FileUploaderTrigger>Choose files</FileUploaderTrigger>
        or drag &amp; drop
      </FileUploaderDropzone>
      <FileUploaderList v-slot="{ files }">
        <FileUploaderItem v-for="file in files" :key="file.name" :file="file">
          {{ file.name }}
          <FileUploaderItemDelete />
        </FileUploaderItem>
      </FileUploaderList>
    </FileUploaderRoot>
  </variant>

  <variant name="Multiple">
    <FileUploaderRoot multiple>
      <FileUploaderDropzone>
        <FileUploaderTrigger>Choose files</FileUploaderTrigger>
        or drag &amp; drop
      </FileUploaderDropzone>
      <FileUploaderList v-slot="{ files }">
        <FileUploaderItem v-for="file in files" :key="file.name" :file="file">
          {{ file.name }}
          <FileUploaderItemDelete />
        </FileUploaderItem>
      </FileUploaderList>
    </FileUploaderRoot>
  </variant>

  <variant name="Disabled">
    <FileUploaderRoot disabled>
      <FileUploaderDropzone>
        <FileUploaderTrigger>Choose files</FileUploaderTrigger>
        or drag &amp; drop
      </FileUploaderDropzone>
      <FileUploaderList v-slot="{ files }">
        <FileUploaderItem v-for="file in files" :key="file.name" :file="file">
          {{ file.name }}
          <FileUploaderItemDelete />
        </FileUploaderItem>
      </FileUploaderList>
    </FileUploaderRoot>
  </variant>
</art>
