# Vize Art - VS Code Extension

Syntax highlighting for Vize Art files (`*.art.vue`).

## Features

- Syntax highlighting for `<art>` and `<variant>` blocks
- Highlighting for Art-specific attributes (`title`, `component`, `description`, etc.)
- Highlighting for variant attributes (`name`, `default`, `args`, etc.)
- Embedded language support for `<script>` (TypeScript) and `<style>` (CSS)
- Vue template syntax highlighting inside variants

## Art File Format

```vue
<!-- Button.art.vue -->
<art title="Button" component="./Button.vue" category="UI">
  <variant name="Primary" default>
    <Button variant="primary">Click me</Button>
  </variant>
  <variant name="Secondary">
    <Button variant="secondary">Click me</Button>
  </variant>
</art>

<script setup lang="ts">
import Button from './Button.vue'
</script>
```

## Installation

### From VSIX

```bash
code --install-extension vize-art-0.0.1-alpha.78.vsix
```

### Development

```bash
cd npm/vscode-art
npm install
npm run compile
```

Then press F5 in VS Code to launch the Extension Development Host.

## Supported Attributes

### Art Block (`<art>`)
- `title` - Component title (required)
- `component` - Path to component file
- `description` - Component description
- `category` - Category for grouping
- `tags` - Comma-separated tags
- `status` - Development status (stable, beta, deprecated)
- `order` - Display order

### Variant Block (`<variant>`)
- `name` - Variant name (required)
- `default` - Boolean flag for default variant
- `args` - JSON object with prop values
- `viewport` - Viewport preset
- `skip-vrt` - Skip visual regression testing

## License

MIT
