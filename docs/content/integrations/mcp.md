---
title: MCP Server
---

# MCP Server

> **⚠️ Work in Progress:** Vize is under active development and is not yet ready for production use. MCP server capabilities may change without notice.

Vize provides a [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server for AI-powered development workflows. The MCP server bridges the gap between your component gallery (Musea) and AI assistants, enabling them to understand, navigate, and work with your Vue components.

## Installation

```bash
npm install @vizejs/musea-mcp-server
```

## What is MCP?

The Model Context Protocol is an open standard for connecting AI assistants (like Claude, ChatGPT, and others) to development tools. Instead of AI assistants guessing about your codebase, MCP provides structured access to real component data — props, events, slots, variants, and documentation.

Vize's MCP server exposes component information from the Musea gallery, so your AI assistant has the same understanding of your components that a developer browsing the gallery would have.

## Capabilities

The MCP server provides the following tools to AI assistants:

### Component Discovery

- **List all components** — Browse all registered components with their categories, tags, and status
- **Search components** — Find components by name, tag, or description
- **Get component metadata** — Retrieve detailed information about a specific component

### Component API

- **Props** — Complete prop definitions with types, defaults, and required status
- **Events** — Emitted events with payload types
- **Slots** — Named slots with slot prop types
- **Expose** — Publicly exposed methods and properties

### Story Information

- **Variant listing** — All variants defined in art files
- **Variant source** — Template code for each variant
- **Default variant** — Which variant is shown by default

### Design Tokens

- **Token listing** — All design tokens from the tokens file
- **Token categories** — Colors, typography, spacing, breakpoints
- **Token resolution** — Semantic tokens resolved to their primitive values

## Setup

### With Claude Code

Add the MCP server to your Claude Code configuration:

```json
// .claude/settings.json
{
  "mcpServers": {
    "vize-musea": {
      "command": "npx",
      "args": ["@vizejs/musea-mcp-server"]
    }
  }
}
```

### With Claude Desktop

Add to your Claude Desktop MCP configuration:

```json
{
  "mcpServers": {
    "vize-musea": {
      "command": "npx",
      "args": ["@vizejs/musea-mcp-server"]
    }
  }
}
```

### With Other AI Assistants

Any MCP-compatible AI assistant can use the server. The configuration pattern is the same — point the assistant to `npx @vizejs/musea-mcp-server`.

## Use Cases

### Component Discovery

Ask your AI assistant to find the right component:

> "What button components do we have? Show me the variants for VFButton."

The AI can query the MCP server to find all button-related components, their props, and available variants — then suggest the correct usage.

### Code Generation

Generate component usage with correct props:

> "Create a form with our VFInput and VFTextarea components, including validation error states."

The AI knows the exact prop names, types, and available variants from the MCP server, generating accurate code without hallucinating prop names.

### API Reference

Query component APIs programmatically:

> "What props does VFNameBadgePreview accept? What are the valid values for user-role?"

The AI returns the real prop definitions from your codebase, not generic guesses.

### Documentation Assistance

> "Write documentation for our SponsorGrid component based on its props and variants."

The AI can generate accurate documentation by inspecting the actual component metadata through MCP.

## How It Works

```
AI Assistant
  ↕ MCP Protocol (JSON-RPC over stdio)
@vizejs/musea-mcp-server
  ↕ Reads art files and component sources
Your Project (*.art.vue files + components)
```

The MCP server:

1. Discovers all `*.art.vue` files in your project
2. Parses them using `vize_musea` to extract component metadata
3. Exposes the metadata through MCP tools
4. Responds to AI assistant queries in real-time
