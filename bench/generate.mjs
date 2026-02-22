#!/usr/bin/env node
/**
 * Generate SFC files for benchmarking
 * Usage: node generate.mjs [count]
 */
import { writeFileSync, mkdirSync, readdirSync, statSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const FILE_COUNT = parseInt(process.argv[2]) || 2000;

// SFC templates of varying complexity
const templates = [
  // Simple
  `<template>
  <div>{{ message }}</div>
</template>

<script setup>
import { ref } from 'vue'
const message = ref('Hello World')
</script>
`,
  // With style
  `<template>
  <div class="container">
    <h1>{{ title }}</h1>
    <p>{{ content }}</p>
  </div>
</template>

<script setup>
import { ref } from 'vue'
const title = ref('Title')
const content = ref('Content')
</script>

<style scoped>
.container { padding: 20px; }
h1 { color: #333; }
</style>
`,
  // Complex with v-for and v-if
  `<template>
  <div class="app">
    <header>
      <h1>{{ title }}</h1>
      <nav>
        <a v-for="link in links" :key="link.id" :href="link.url">{{ link.text }}</a>
      </nav>
    </header>
    <main>
      <section v-if="loading">Loading...</section>
      <section v-else>
        <article v-for="item in items" :key="item.id">
          <h2>{{ item.title }}</h2>
          <p>{{ item.body }}</p>
          <button @click="selectItem(item)">Select</button>
        </article>
      </section>
    </main>
    <footer><p>&copy; {{ year }}</p></footer>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
const title = ref('My App')
const loading = ref(false)
const items = ref([])
const links = ref([{ id: 1, url: '/', text: 'Home' }, { id: 2, url: '/about', text: 'About' }])
const year = computed(() => new Date().getFullYear())
function selectItem(item) { console.log('Selected:', item) }
</script>

<style scoped>
.app { max-width: 1200px; margin: 0 auto; }
header { display: flex; justify-content: space-between; }
</style>
`,
  // Dashboard with many bindings
  `<template>
  <div class="dashboard">
    <aside class="sidebar">
      <div class="logo"><img :src="logoUrl" :alt="appName" /><span>{{ appName }}</span></div>
      <nav class="nav-menu">
        <ul>
          <li v-for="item in menuItems" :key="item.id" :class="{ active: item.active }">
            <a :href="item.href" @click.prevent="navigate(item)">
              <span class="icon">{{ item.icon }}</span>
              <span class="label">{{ item.label }}</span>
              <span v-if="item.badge" class="badge">{{ item.badge }}</span>
            </a>
          </li>
        </ul>
      </nav>
    </aside>
    <main class="main-content">
      <section class="stats-grid">
        <div v-for="stat in stats" :key="stat.id" class="stat-card" :style="{ borderColor: stat.color }">
          <div class="stat-icon" :style="{ backgroundColor: stat.color }">{{ stat.icon }}</div>
          <div class="stat-info">
            <span class="stat-value">{{ stat.value }}</span>
            <span class="stat-label">{{ stat.label }}</span>
          </div>
        </div>
      </section>
      <section class="data-table">
        <table>
          <thead><tr><th v-for="col in columns" :key="col.key" @click="sortBy(col.key)">{{ col.label }}</th></tr></thead>
          <tbody><tr v-for="row in paginatedData" :key="row.id"><td v-for="col in columns" :key="col.key">{{ row[col.key] }}</td></tr></tbody>
        </table>
        <div class="pagination">
          <button @click="prevPage" :disabled="currentPage === 1">Prev</button>
          <span>Page {{ currentPage }} of {{ totalPages }}</span>
          <button @click="nextPage" :disabled="currentPage === totalPages">Next</button>
        </div>
      </section>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, onMounted } from 'vue'
const appName = ref('Dashboard')
const logoUrl = ref('/logo.png')
const currentPage = ref(1)
const pageSize = ref(10)
const menuItems = ref([
  { id: 1, label: 'Dashboard', icon: 'Chart', href: '/', active: true },
  { id: 2, label: 'Users', icon: 'Users', href: '/users', badge: 5 },
])
const stats = ref([
  { id: 1, label: 'Users', value: '12,345', icon: 'Users', color: '#4CAF50' },
  { id: 2, label: 'Revenue', value: '$54,321', icon: 'Money', color: '#2196F3' },
])
const columns = ref([{ key: 'id', label: 'ID' }, { key: 'name', label: 'Name' }])
const tableData = ref([])
const totalPages = computed(() => Math.ceil(tableData.value.length / pageSize.value))
const paginatedData = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return tableData.value.slice(start, start + pageSize.value)
})
function navigate(item) { menuItems.value.forEach(i => i.active = i.id === item.id) }
function sortBy(key) { console.log('Sort by', key) }
function prevPage() { if (currentPage.value > 1) currentPage.value-- }
function nextPage() { if (currentPage.value < totalPages.value) currentPage.value++ }
onMounted(() => { tableData.value = Array.from({ length: 50 }, (_, i) => ({ id: i + 1, name: 'Item ' + (i + 1) })) })
</script>

<style scoped>
.dashboard { display: flex; min-height: 100vh; }
.sidebar { width: 260px; background: #1a1a2e; color: white; padding: 20px; }
.main-content { flex: 1; padding: 20px; background: #f5f5f5; }
.stats-grid { display: grid; grid-template-columns: repeat(4, 1fr); gap: 20px; }
</style>
`,
  // Product page with complex interactions
  `<template>
  <div class="product-page">
    <div class="product-container">
      <div class="product-gallery">
        <div class="main-image">
          <img :src="selectedImage" :alt="product.name" />
          <button @click="prevImage" class="nav-btn prev">Prev</button>
          <button @click="nextImage" class="nav-btn next">Next</button>
        </div>
        <div class="thumbnails">
          <button v-for="(img, index) in product.images" :key="index" @click="selectImage(index)" :class="{ active: selectedImageIndex === index }">
            <img :src="img.thumbnail" :alt="img.alt" />
          </button>
        </div>
      </div>
      <div class="product-info">
        <span class="brand">{{ product.brand }}</span>
        <h1 class="title">{{ product.name }}</h1>
        <div class="rating">
          <span v-for="star in 5" :key="star" class="star" :class="{ filled: star <= product.rating }">Star</span>
          <span class="count">({{ product.reviewCount }} reviews)</span>
        </div>
        <div class="price-section">
          <span v-if="product.originalPrice" class="original-price">\${{ product.originalPrice }}</span>
          <span class="current-price">\${{ product.price }}</span>
        </div>
        <div class="options">
          <div class="color-options">
            <button v-for="color in product.colors" :key="color.name" @click="selectColor(color)" :class="{ selected: selectedColor === color.name }" :style="{ backgroundColor: color.hex }"></button>
          </div>
          <div class="size-options">
            <button v-for="size in product.sizes" :key="size" @click="selectSize(size)" :class="{ selected: selectedSize === size }">{{ size }}</button>
          </div>
          <div class="quantity-selector">
            <button @click="decrementQuantity" :disabled="quantity <= 1">-</button>
            <input v-model.number="quantity" type="number" min="1" :max="product.stock" />
            <button @click="incrementQuantity" :disabled="quantity >= product.stock">+</button>
          </div>
        </div>
        <div class="actions">
          <button @click="addToCart" class="add-to-cart" :disabled="!canAddToCart">Add to Cart - \${{ totalPrice }}</button>
          <button @click="toggleWishlist" class="wishlist" :class="{ active: isWishlisted }">Heart</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue'
const product = ref({
  name: 'Premium Headphones', brand: 'AudioTech', price: 299.99, originalPrice: 399.99, rating: 4.5, reviewCount: 1234, stock: 15,
  images: [{ full: '/img1.jpg', thumbnail: '/t1.jpg', alt: 'Front' }],
  colors: [{ name: 'Black', hex: '#000' }, { name: 'White', hex: '#FFF' }],
  sizes: ['S', 'M', 'L', 'XL']
})
const selectedImageIndex = ref(0)
const selectedColor = ref('Black')
const selectedSize = ref('M')
const quantity = ref(1)
const isWishlisted = ref(false)
const selectedImage = computed(() => product.value.images[selectedImageIndex.value]?.full)
const totalPrice = computed(() => (product.value.price * quantity.value).toFixed(2))
const canAddToCart = computed(() => selectedColor.value && selectedSize.value && quantity.value > 0)
function selectImage(index) { selectedImageIndex.value = index }
function prevImage() { selectedImageIndex.value = (selectedImageIndex.value - 1 + product.value.images.length) % product.value.images.length }
function nextImage() { selectedImageIndex.value = (selectedImageIndex.value + 1) % product.value.images.length }
function selectColor(color) { selectedColor.value = color.name }
function selectSize(size) { selectedSize.value = size }
function incrementQuantity() { if (quantity.value < product.value.stock) quantity.value++ }
function decrementQuantity() { if (quantity.value > 1) quantity.value-- }
function addToCart() { console.log('Add to cart') }
function toggleWishlist() { isWishlisted.value = !isWishlisted.value }
</script>

<style scoped>
.product-page { max-width: 1400px; margin: 0 auto; padding: 20px; }
.product-container { display: grid; grid-template-columns: 1fr 1fr; gap: 40px; }
.thumbnails { display: flex; gap: 10px; margin-top: 10px; }
.price-section { font-size: 24px; margin: 20px 0; }
.original-price { text-decoration: line-through; color: #999; }
.current-price { color: #e74c3c; font-weight: bold; }
</style>
`,
  // Chat with real-time features
  `<template>
  <div class="chat-container">
    <aside class="conversations-sidebar">
      <div class="search-box"><input v-model="searchQuery" type="text" placeholder="Search..." /></div>
      <ul class="conversation-list">
        <li v-for="conv in filteredConversations" :key="conv.id" :class="{ active: activeConversation?.id === conv.id }" @click="selectConversation(conv)">
          <div class="avatar" :class="{ online: conv.participant.online }"><img :src="conv.participant.avatar" :alt="conv.participant.name" /></div>
          <div class="conv-info">
            <span class="name">{{ conv.participant.name }}</span>
            <span class="time">{{ formatTime(conv.lastMessage.timestamp) }}</span>
            <span v-if="conv.typing" class="typing">typing...</span>
            <span v-else class="last-message">{{ conv.lastMessage.text }}</span>
          </div>
        </li>
      </ul>
    </aside>
    <main class="chat-main">
      <template v-if="activeConversation">
        <header class="chat-header">
          <div class="avatar" :class="{ online: activeConversation.participant.online }"><img :src="activeConversation.participant.avatar" /></div>
          <span class="name">{{ activeConversation.participant.name }}</span>
          <button @click="startCall('voice')">Phone</button>
          <button @click="startCall('video')">Video</button>
        </header>
        <div class="messages-container" ref="messagesContainer">
          <template v-for="(group, date) in groupedMessages" :key="date">
            <div class="date-divider">{{ formatDate(date) }}</div>
            <div v-for="message in group" :key="message.id" class="message" :class="{ sent: message.senderId === currentUser.id }">
              <div class="message-content">
                <div v-if="message.type === 'text'" class="text-message">{{ message.text }}</div>
                <div v-else-if="message.type === 'image'" class="image-message"><img :src="message.imageUrl" @click="openImage(message)" /></div>
                <div class="message-meta">
                  <span class="time">{{ formatTime(message.timestamp) }}</span>
                  <span v-if="message.senderId === currentUser.id" class="status">{{ message.status }}</span>
                </div>
              </div>
              <div class="message-actions">
                <button @click="replyTo(message)">Reply</button>
              </div>
            </div>
          </template>
        </div>
        <footer class="chat-input">
          <button @click="showAttachMenu = !showAttachMenu">Attach</button>
          <textarea v-model="messageText" placeholder="Type a message..." @keydown.enter.exact.prevent="sendMessage"></textarea>
          <button v-if="messageText.trim()" @click="sendMessage">Send</button>
        </footer>
      </template>
    </main>
  </div>
</template>

<script setup>
import { ref, computed, nextTick } from 'vue'
const currentUser = ref({ id: 1, name: 'Me', avatar: '/me.jpg' })
const conversations = ref([
  { id: 1, participant: { id: 2, name: 'Alice', avatar: '/alice.jpg', online: true }, lastMessage: { text: 'Hey!', timestamp: new Date() }, typing: false },
  { id: 2, participant: { id: 3, name: 'Bob', avatar: '/bob.jpg', online: false }, lastMessage: { text: 'See you!', timestamp: new Date(Date.now() - 7200000) }, typing: false },
])
const messages = ref([
  { id: 1, conversationId: 1, senderId: 2, type: 'text', text: 'Hey!', timestamp: new Date(Date.now() - 86400000), status: 'read' },
  { id: 2, conversationId: 1, senderId: 1, type: 'text', text: 'Hi Alice!', timestamp: new Date(Date.now() - 86300000), status: 'read' },
])
const searchQuery = ref('')
const activeConversation = ref(null)
const messageText = ref('')
const showAttachMenu = ref(false)
const messagesContainer = ref(null)
const filteredConversations = computed(() => searchQuery.value ? conversations.value.filter(c => c.participant.name.toLowerCase().includes(searchQuery.value.toLowerCase())) : conversations.value)
const currentMessages = computed(() => activeConversation.value ? messages.value.filter(m => m.conversationId === activeConversation.value.id) : [])
const groupedMessages = computed(() => { const g = {}; currentMessages.value.forEach(m => { const d = new Date(m.timestamp).toDateString(); if (!g[d]) g[d] = []; g[d].push(m) }); return g })
function selectConversation(conv) { activeConversation.value = conv; nextTick(() => scrollToBottom()) }
function sendMessage() { if (!messageText.value.trim()) return; messages.value.push({ id: Date.now(), conversationId: activeConversation.value.id, senderId: currentUser.value.id, type: 'text', text: messageText.value, timestamp: new Date(), status: 'sent' }); messageText.value = ''; nextTick(() => scrollToBottom()) }
function scrollToBottom() { if (messagesContainer.value) messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight }
function formatTime(date) { return new Date(date).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) }
function formatDate(date) { return new Date(date).toLocaleDateString() }
function startCall(type) { console.log('Start', type, 'call') }
function replyTo(message) { console.log('Reply to', message.id) }
function openImage(message) { console.log('Open image', message.imageUrl) }
</script>

<style scoped>
.chat-container { display: grid; grid-template-columns: 320px 1fr; height: 100vh; }
.conversations-sidebar { background: #f8f9fa; border-right: 1px solid #e0e0e0; }
.conversation-list li { display: flex; padding: 15px; cursor: pointer; border-bottom: 1px solid #eee; }
.conversation-list li.active { background: #e3f2fd; }
.chat-main { display: flex; flex-direction: column; }
.messages-container { flex: 1; overflow-y: auto; padding: 20px; }
.message { display: flex; margin-bottom: 15px; max-width: 70%; }
.message.sent { margin-left: auto; }
.message-content { background: #f1f1f1; padding: 10px 15px; border-radius: 18px; }
.message.sent .message-content { background: #0084ff; color: white; }
.chat-input { display: flex; align-items: center; padding: 15px; border-top: 1px solid #e0e0e0; gap: 10px; }
.chat-input textarea { flex: 1; padding: 10px; border: 1px solid #ddd; border-radius: 20px; resize: none; }
</style>
`,
];

const benchDir = join(__dirname, '__in__');

// Ensure directory exists
mkdirSync(benchDir, { recursive: true });

console.log(`Generating ${FILE_COUNT} SFC files in ${benchDir}...`);

for (let i = 0; i < FILE_COUNT; i++) {
  const template = templates[i % templates.length];
  const filename = `Component${String(i).padStart(4, '0')}.vue`;
  const filepath = join(benchDir, filename);
  writeFileSync(filepath, template);

  if ((i + 1) % 500 === 0) {
    console.log(`  Generated ${i + 1} files...`);
  }
}

console.log(`Done! Generated ${FILE_COUNT} SFC files.`);

// Calculate total size
const files = readdirSync(benchDir).filter(f => f.endsWith('.vue'));
const totalSize = files.reduce((sum, f) => sum + statSync(join(benchDir, f)).size, 0);
console.log(`Total size: ${(totalSize / 1024 / 1024).toFixed(2)} MB`);

// Generate tsconfig.json for vue-tsc / vize check
const tsconfig = {
  compilerOptions: {
    target: "ESNext",
    module: "ESNext",
    moduleResolution: "bundler",
    strict: true,
    jsx: "preserve",
    noEmit: true,
    skipLibCheck: true,
    paths: {
      "vue": ["../node_modules/vue"]
    }
  },
  include: ["./*.vue"]
};
writeFileSync(join(benchDir, 'tsconfig.json'), JSON.stringify(tsconfig, null, 2));
console.log('Generated tsconfig.json');

// Generate eslint.config.mjs for eslint-plugin-vue
const eslintConfig = `import pluginVue from "eslint-plugin-vue";

export default [
  ...pluginVue.configs["flat/recommended"],
  {
    files: ["*.vue"],
    rules: {
      "vue/multi-word-component-names": "off",
    },
  },
];
`;
writeFileSync(join(benchDir, 'eslint.config.mjs'), eslintConfig);
console.log('Generated eslint.config.mjs');

// Generate vite entry file for vite-plugin benchmark
const viteEntryImports = [];
const viteEntryComponents = [];
const entryCount = FILE_COUNT; // import all files for fair vite benchmark
for (let i = 0; i < entryCount; i++) {
  const name = `Component${String(i).padStart(4, '0')}`;
  viteEntryImports.push(`import ${name} from './${name}.vue'`);
  viteEntryComponents.push(name);
}
const viteEntry = `${viteEntryImports.join('\n')}
import { createApp, h } from 'vue'

const app = createApp({
  render() {
    return h('div', [${viteEntryComponents.map(c => `h(${c})`).join(', ')}])
  }
})
app.mount('#app')
`;
writeFileSync(join(benchDir, 'main.ts'), viteEntry);
console.log(`Generated main.ts (imports ${entryCount} components)`);

// Generate index.html for vite
const indexHtml = `<!DOCTYPE html>
<html>
<head><title>Bench</title></head>
<body>
  <div id="app"></div>
  <script type="module" src="./main.ts"><\/script>
</body>
</html>
`;
writeFileSync(join(benchDir, 'index.html'), indexHtml);
console.log('Generated index.html');
