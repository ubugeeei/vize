# Vize

High-performance Vue.js toolchain in Rust.

## Installation

```bash
# Install from npm
npm install -g vize

# Or install from GitHub
npm install -g github:ubugeeei/vize
```

## Usage

```bash
vize [COMMAND] [OPTIONS]
```

| Command | Description                            |
| ------- | -------------------------------------- |
| `build` | Compile Vue SFC files (default)        |
| `fmt`   | Format Vue SFC files                   |
| `lint`  | Lint Vue SFC files                     |
| `check` | Type check Vue SFC files               |
| `musea` | Start component gallery server         |
| `lsp`   | Start Language Server Protocol server  |

```bash
vize --help           # Show help
vize <command> --help # Show command-specific help
```

## Examples

```bash
vize                              # Compile ./**/*.vue to ./dist
vize build src/**/*.vue -o out    # Custom input/output
vize build --ssr                  # SSR mode
vize fmt --check                  # Check formatting
vize lint --fix                   # Auto-fix lint issues
vize check --strict               # Strict type checking
```

## Alternative Installation

If npm installation fails, you can install via Cargo:

```bash
cargo install vize
```

## License

MIT
