# Fresco SFC Example

Vue Single File Component (SFC) を使った Fresco の例です。
`<script setup>` 構文をサポートしています。

## セットアップ

```bash
pnpm install
```

## ビルド

```bash
pnpm build
```

## 実行

```bash
node dist/main.js
```

## SFC の書き方

### `<script setup>` (推奨)

```vue
<script setup lang="ts">
import { ref } from 'vue';

const count = ref(0);
</script>

<template>
  <box :border="'single'" :padding="1">
    <text :bold="true" :fg="'green'">Count: {{ count }}</text>
  </box>
</template>
```

### Props と Emits

```vue
<script setup lang="ts">
import { ref, computed } from 'vue';

const props = withDefaults(defineProps<{
  initialValue?: number;
  fg?: string;
}>(), {
  initialValue: 0,
});

const emit = defineEmits<{
  change: [value: number];
}>();

const count = ref(props.initialValue);

const increment = () => {
  count.value++;
  emit('change', count.value);
};
</script>

<template>
  <box>
    <text @click="increment">Count: {{ count }}</text>
  </box>
</template>
```

### Fresco コンポーネントの使用

```vue
<script setup lang="ts">
import { Spinner, ProgressBar } from '@vizejs/fresco';

const progress = ref(50);
</script>

<template>
  <box>
    <Spinner type="dots" />
    <ProgressBar :value="progress" :width="20" />
  </box>
</template>
```

### Render Function ベース

より細かい制御が必要な場合：

```vue
<script lang="ts">
import { defineComponent, h } from 'vue';
import { Box, Text } from '@vizejs/fresco';

export default defineComponent({
  setup() {
    return () => h(Box, { border: 'single' }, [
      h(Text, { bold: true }, 'Hello')
    ]);
  },
});
</script>
```

## Vite 設定のポイント

```ts
// vite.config.ts
import { vize } from '@vizejs/vite-plugin';

export default defineConfig({
  plugins: [
    vize(),
  ],
  resolve: {
    alias: {
      // SSR shim（script setup 用）
      '@vue/runtime-core/server-renderer': resolve(__dirname, 'ssr-shim.ts'),
    },
  },
});
```

## ファイル構成

- `main.ts` - エントリーポイント
- `App.vue` - メインコンポーネント（script setup）
- `Counter.vue` - カウンターコンポーネント（script setup）
- `Dashboard.vue` - ダッシュボード（Render Function）
- `ssr-shim.ts` - SSR シム
- `vite.config.ts` - Vite 設定
