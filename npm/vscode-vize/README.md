# Vize - VS Code Extension

Vue Language Support powered by Vize - A high-performance language server for Vue SFC.

## Features

- **Diagnostics** - Real-time error detection
- **Completion** - Vue directives, components, Composition API
- **Hover** - Type information and documentation
- **Go to Definition** - Navigate template to script
- **Find References** - Cross-file reference search
- **Rename** - Safe identifier renaming
- **Semantic Highlighting** - Vue-specific syntax colors
- **Code Lens** - Reference counts

## Installation

### From VS Code Marketplace

Search "Vize" in VS Code Extensions.

### From VSIX

```bash
code --install-extension vize-0.0.1-alpha.78.vsix
```

### Development

```bash
cd npm/vscode-vize
pnpm install
pnpm run build
# Press F5 to launch Extension Development Host
```

## Requirements

- VS Code 1.75+
- `vize` CLI installed (`cargo install vize`)

## Configuration

```json
{
  "vize.enable": true,
  "vize.serverPath": "",
  "vize.trace.server": "off",
  "vize.diagnostics.enable": true,
  "vize.completion.enable": true,
  "vize.hover.enable": true,
  "vize.codeLens.enable": true
}
```

## Commands

- `Vize: Restart Language Server` - Restart the LSP server
- `Vize: Show Output Channel` - Show server logs

## License

MIT
