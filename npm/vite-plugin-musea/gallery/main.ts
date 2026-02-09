import { createApp } from 'vue'
import App from './App.vue'
import { router } from './router'
import './styles/gallery.css'
import 'highlight.js/styles/github-dark.css'
import './styles/hljs-light.css'

const app = createApp(App)
app.use(router)
app.mount('#app')
