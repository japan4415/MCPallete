# MCPallete

MCPallete is a 100% live-coded Rust TUI tool for managing multiple MCP server configurations and presets interactively. It was developed entirely through iterative coding and direct feedback, with no code written outside of the live-coding process.

## Features
- Interactive TUI for enabling/disabling MCP servers per environment
- Preset save, apply, and delete functionality
- Environment-specific configPath support (e.g., for Claude Desktop)
- Automatic generation of environment-specific config files (with env var expansion)
- Supports $VAR and ${VAR} style environment variable expansion in config
- All logic, UI, and features were implemented via live-coding only

## Usage
- Start with `cargo run --release`
- Use arrow keys, Tab, and Space to navigate and toggle
- Ctrl+S: Save current state (enable/preset/configPath)
- Ctrl+R: Reload config
- Ctrl+D: Delete selected preset
- Ctrl+C: Exit

## Configuration Example
See `~/.config/mcpallete/basic_config.json` for structure. Example:

```json
{
  "mcpServers": {
    "firecrawl-mcp": {
      "command": "npx",
      "args": ["-y", "firecrawl-mcp"],
      "env": {
        "FIRECRAWL_API_KEY": "$FIRECRAWL_API_KEY"
      }
    }
  },
  "environments": {
    "claudeDescktop": {
      "configPath": "/path/to/claude_desktop_config.json",
      "enable": ["firecrawl-mcp"],
      "preset": {"default": ["firecrawl-mcp"]},
      "mode": "claude_desktop"
    }
  }
}
```

## License
MIT