/**
 * Fresco Demo - Showcase of components
 *
 * Run with: npx tsx examples/demo.ts
 */

import { h, ref, defineComponent } from '@vue/runtime-core';
import {
  createApp,
  Box,
  Text,
  Spinner,
  ProgressBar,
  Select,
  Checkbox,
  Table,
  Tabs,
  Divider,
} from '../src/index.js';

const Demo = defineComponent({
  setup() {
    // State
    const activeTab = ref('components');
    const selectedColor = ref('blue');
    const checked = ref(false);
    const progress = ref(0);

    // Simulate progress
    setInterval(() => {
      progress.value = (progress.value + 5) % 105;
    }, 200);

    // Color options
    const colorOptions = [
      { label: 'Blue', value: 'blue' },
      { label: 'Green', value: 'green' },
      { label: 'Red', value: 'red' },
      { label: 'Yellow', value: 'yellow' },
    ];

    // Table data
    const tableData = [
      { name: 'Alice', role: 'Developer', status: 'Active' },
      { name: 'Bob', role: 'Designer', status: 'Away' },
      { name: 'Charlie', role: 'Manager', status: 'Active' },
    ];

    const tableColumns = [
      { key: 'name', header: 'Name', width: 12 },
      { key: 'role', header: 'Role', width: 12 },
      { key: 'status', header: 'Status', width: 10 },
    ];

    // Tabs
    const tabs = [
      { key: 'components', label: 'Components' },
      { key: 'table', label: 'Table' },
      { key: 'about', label: 'About' },
    ];

    return () =>
      h(Box, { flexDirection: 'column', padding: 1 }, [
        // Header
        h(Box, { key: 'header', justifyContent: 'center', marginBottom: 1 }, [
          h(Text, { bold: true, fg: 'cyan' }, 'Fresco Demo'),
        ]),

        // Tabs
        h(Tabs, {
          key: 'tabs',
          tabs,
          modelValue: activeTab.value,
          'onUpdate:modelValue': (v: string) => (activeTab.value = v),
        }, () => {
          // Tab content
          if (activeTab.value === 'components') {
            return h(Box, { flexDirection: 'column', gap: 1, marginTop: 1 }, [
              // Spinner
              h(Box, { key: 'spinner-section', flexDirection: 'row', gap: 1 }, [
                h(Text, {}, 'Loading:'),
                h(Spinner, { type: 'dots' }),
              ]),

              h(Divider, { key: 'div1' }),

              // Progress Bar
              h(Box, { key: 'progress-section', flexDirection: 'column' }, [
                h(Text, {}, 'Progress:'),
                h(ProgressBar, { value: progress.value, width: 30 }),
              ]),

              h(Divider, { key: 'div2' }),

              // Select
              h(Box, { key: 'select-section', flexDirection: 'column' }, [
                h(Text, {}, 'Select a color:'),
                h(Select, {
                  options: colorOptions,
                  modelValue: selectedColor.value,
                }),
              ]),

              h(Divider, { key: 'div3' }),

              // Checkbox
              h(Checkbox, {
                key: 'checkbox',
                label: 'Enable feature',
                modelValue: checked.value,
              }),
            ]);
          }

          if (activeTab.value === 'table') {
            return h(Box, { marginTop: 1 }, [
              h(Table, {
                columns: tableColumns,
                data: tableData,
                border: 'single',
              }),
            ]);
          }

          if (activeTab.value === 'about') {
            return h(Box, { flexDirection: 'column', marginTop: 1 }, [
              h(Text, { bold: true }, 'Fresco'),
              h(Text, {}, 'Vue TUI Framework'),
              h(Text, { dim: true }, 'Build terminal UIs with Vue.js'),
            ]);
          }

          return null;
        }),

        // Footer
        h(Box, { key: 'footer', marginTop: 2 }, [
          h(Text, { dim: true }, 'Press Ctrl+C to exit'),
        ]),
      ]);
  },
});

// Create and run the app
const app = createApp(Demo, {
  exitOnCtrlC: true,
});

void app.mount();
void app.waitUntilExit().then(() => {
  console.log('Demo ended');
});
