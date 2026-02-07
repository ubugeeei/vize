<script setup>
import { ConfigProvider } from './index'
</script>

<art title="ConfigProvider" component="./ConfigProvider.vue" category="Utilities" status="ready" tags="config,provider,direction">
  <variant name="Default" default>
    <ConfigProvider>
      <p>Content with default LTR direction</p>
    </ConfigProvider>
  </variant>

  <variant name="RTL Direction">
    <ConfigProvider dir="rtl">
      <p>Content with RTL direction</p>
    </ConfigProvider>
  </variant>
</art>
