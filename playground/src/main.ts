import { createApp } from 'vue';
import App from './App.vue';
import './styles.css';
// Import vize component styles (extracted CSS in production)
import 'virtual:vize-styles';

// Monaco Editor worker configuration for Vite
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker';

self.MonacoEnvironment = {
  getWorker(_, label) {
    if (label === 'typescript' || label === 'javascript') {
      return new tsWorker();
    }
    return new editorWorker();
  },
};

createApp(App).mount('#app');
