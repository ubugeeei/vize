<script setup lang="ts">
const props = defineProps<{
  title: string;
  description?: string;
  siteName?: string;
}>();
</script>

<template>
  <div class="og">
    <div class="left">
      <div class="header">
        <img class="header-logo" src="/logo.svg" width="28" height="28" />
        <span class="site-name">VIZE</span>
      </div>
      <div class="title-area">
        <h1 class="title">{{ props.title }}</h1>
        <p class="desc">{{ props.description || 'Unofficial High-Performance Vue.js Toolchain in Rust' }}</p>
      </div>
      <span class="url">vizejs.dev</span>
    </div>
    <div class="right">
      <img class="logo" src="/logo.svg" />
      <span class="tagline">High-Performance Vue.js Toolchain in Rust</span>
    </div>
  </div>
</template>

<style scoped>
.og {
  width: 1200px;
  height: 630px;
  background: #e6e2d6;
  position: relative;
  overflow: hidden;
  font-family: 'Helvetica Neue', Helvetica, Arial, system-ui, sans-serif;
  color: #121212;
  display: flex;
}

.left {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 56px 0 56px 72px;
  position: relative;
  z-index: 1;
}

.right {
  width: 420px;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
}

.header {
  display: flex;
  align-items: center;
  gap: 12px;
}

.header-logo {
  width: 28px;
  height: 28px;
  object-fit: contain;
}

.site-name {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: 0.2em;
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  color: #5a5750;
}

.title-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
}

.title {
  font-size: 96px;
  font-weight: 700;
  letter-spacing: -0.06em;
  line-height: 1.15;
  margin: 0;
  padding-bottom: 4px;
  max-width: 680px;
}

.desc {
  font-size: 21px;
  font-weight: 500;
  color: #5a5750;
  line-height: 1.35;
  letter-spacing: -0.02em;
  margin: 32px 0 0 0;
  max-width: 480px;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.url {
  font-size: 14px;
  font-weight: 500;
  color: #5a5750;
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  letter-spacing: 0.02em;
}

.logo {
  width: auto;
  height: 340px;
  opacity: 0.85;
  position: absolute;
  right: -16px;
  top: 50%;
  transform: translateY(-50%);
}

.tagline {
  position: absolute;
  right: 40px;
  bottom: 56px;
  font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
  font-size: 10px;
  font-weight: 400;
  letter-spacing: 0.02em;
  line-height: 1.6;
  color: #5a5750;
  writing-mode: vertical-rl;
  text-orientation: mixed;
  max-height: 240px;
  opacity: 0.5;
}
</style>
