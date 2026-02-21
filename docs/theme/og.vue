<script setup lang="ts">
const props = defineProps<{
  title: string;
  description?: string;
  siteName?: string;
}>();
</script>

<template>
  <div class="og">
    <div class="bg">
      <div class="vein vein-1"></div>
      <div class="vein vein-2"></div>
      <div class="vein vein-3"></div>
      <div class="grain"></div>
    </div>
    <div class="content">
      <div class="logo">
        <svg viewBox="0 0 100 100" width="48" height="48" fill="none">
          <path d="M20 80L50 10L80 80H65L50 40L35 80H20Z" fill="#e8e4dc" />
          <line x1="55" y1="55" x2="85" y2="55" stroke="#e8e4dc" stroke-width="3" />
          <line x1="55" y1="65" x2="80" y2="65" stroke="#e8e4dc" stroke-width="3" opacity="0.6" />
          <line x1="55" y1="75" x2="72" y2="75" stroke="#e8e4dc" stroke-width="3" opacity="0.3" />
        </svg>
        <span class="logo-text">Vize</span>
      </div>
      <h1 class="title">{{ props.title }}</h1>
      <p v-if="props.description" class="desc">{{ props.description }}</p>
      <div class="footer">
        <span class="tagline">High-Performance Vue.js Toolchain in Rust</span>
        <span class="url">vizejs.dev</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.og {
  width: 1200px;
  height: 630px;
  background: #141414;
  position: relative;
  overflow: hidden;
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  color: #e8e4dc;
}

/* Marble veining effect via CSS */
.bg {
  position: absolute;
  inset: 0;
  opacity: 0.08;
}

.vein {
  position: absolute;
  inset: 0;
}

.vein-1 {
  background:
    repeating-linear-gradient(
      37deg,
      transparent,
      transparent 80px,
      rgba(232, 228, 220, 0.3) 80px,
      rgba(232, 228, 220, 0.08) 82px,
      transparent 84px
    );
}

.vein-2 {
  background:
    repeating-linear-gradient(
      -25deg,
      transparent,
      transparent 120px,
      rgba(232, 228, 220, 0.2) 120px,
      rgba(232, 228, 220, 0.05) 121px,
      transparent 123px
    );
}

.vein-3 {
  background:
    repeating-linear-gradient(
      62deg,
      transparent,
      transparent 200px,
      rgba(232, 228, 220, 0.15) 200px,
      rgba(232, 228, 220, 0.03) 202px,
      transparent 206px
    );
}

.grain {
  position: absolute;
  inset: 0;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E");
  opacity: 0.4;
}

.content {
  position: relative;
  z-index: 1;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  height: 100%;
  padding: 56px 72px;
}

.logo {
  display: flex;
  align-items: center;
  gap: 14px;
}

.logo-text {
  font-size: 28px;
  font-weight: 700;
  letter-spacing: -0.02em;
  color: #e8e4dc;
}

.title {
  font-size: 64px;
  font-weight: 800;
  letter-spacing: -0.04em;
  line-height: 1.1;
  margin: 0;
  max-width: 900px;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.desc {
  font-size: 24px;
  font-weight: 400;
  color: #8a8780;
  line-height: 1.4;
  margin: 0;
  margin-top: 16px;
  max-width: 700px;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.footer {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
}

.tagline {
  font-size: 18px;
  font-weight: 400;
  color: #5a5855;
  letter-spacing: 0.02em;
}

.url {
  font-size: 18px;
  font-weight: 500;
  color: #6b6b6b;
  font-family: 'JetBrains Mono', ui-monospace, monospace;
}
</style>
