/**
 * Simple debug example
 */

import { h, defineComponent } from '@vue/runtime-core';
import { createApp } from '../src/index.js';

const App = defineComponent({
  setup() {
    return () => h('box', {
      style: { flexDirection: 'column', padding: 1 },
      border: 'single'
    }, [
      h('text', { bold: true, fg: 'green' }, 'Hello Fresco!'),
      h('text', {}, 'Simple test'),
      h('text', { dim: true }, 'Press Ctrl+C to exit'),
    ]);
  },
});

const app = createApp(App, {
  exitOnCtrlC: true,
  debug: true,
});

console.log('Starting...');

app.mount().then(() => {
  console.log('Mounted!');
}).catch((err) => {
  console.error('Mount error:', err);
});

void app.waitUntilExit().then(() => {
  console.log('Goodbye!');
});
