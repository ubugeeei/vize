/**
 * Musea Playground Preset
 *
 * This preset demonstrates the Musea story format:
 * - <art> block for component documentation
 * - Multiple <variant> blocks for different component states
 * - CSS custom properties for design tokens
 *
 * Used to showcase story parsing and CSF generation.
 *
 * Note: This file is separate from the Vue component to avoid
 * linting issues with embedded Vue code in template literals.
 */

export const ART_PRESET = `<script setup lang="ts">
import Button from './Button.vue'
</script>

<art
  title="Button"
  description="A versatile button component"
  component="./Button.vue"
  category="atoms"
  tags="ui,input"
>
  <variant name="Primary" default>
    <Button variant="primary">Click me</Button>
  </variant>

  <variant name="Secondary">
    <Button variant="secondary">Click me</Button>
  </variant>

  <variant name="With Icon">
    <Button variant="primary" icon="plus">Add Item</Button>
  </variant>

  <variant name="Disabled">
    <Button variant="primary" disabled>Disabled</Button>
  </variant>
</art>

<style>
:root {
  --color-primary: #3b82f6;
  --color-primary-hover: #2563eb;
  --color-secondary: #6b7280;
  --color-secondary-hover: #4b5563;
  --color-success: #10b981;
  --color-warning: #f59e0b;
  --color-error: #ef4444;
  --color-text: #1f2937;
  --color-text-muted: #6b7280;
  --color-background: #ffffff;
  --color-border: #e5e7eb;

  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;

  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;

  --font-size-sm: 12px;
  --font-size-md: 14px;
  --font-size-lg: 16px;
}
</style>
`;
