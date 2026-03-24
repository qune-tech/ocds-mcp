<!-- mcp-name: io.github.qune-tech/ocds-mcp -->

# @qune-tech/ocds-mcp

MCP server for German public procurement data (OCDS). Connects your AI assistant to the [Vergabe Dashboard](https://vergabe-dashboard.qune.de) API for semantic search, tender matching, and company profile management.

Company profiles never leave your machine. GDPR-compliant by design.

## Quick Start

```bash
npx @qune-tech/ocds-mcp --api-key sk_live_YOUR_KEY_HERE
```

### Get an API key

Sign up at [vergabe-dashboard.qune.de](https://vergabe-dashboard.qune.de) and create an API key (MCP or Enterprise plan required).

## Configure your AI client

### Claude Desktop

Edit `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "ocds": {
      "command": "npx",
      "args": ["-y", "@qune-tech/ocds-mcp", "--api-key", "sk_live_YOUR_KEY_HERE"]
    }
  }
}
```

### Claude Code

Add `.mcp.json` to your project root:

```json
{
  "mcpServers": {
    "ocds": {
      "command": "npx",
      "args": ["-y", "@qune-tech/ocds-mcp", "--api-key", "sk_live_YOUR_KEY_HERE"]
    }
  }
}
```

### Cursor

Settings > MCP Servers > Add:

- Command: `npx`
- Args: `-y @qune-tech/ocds-mcp --api-key sk_live_YOUR_KEY_HERE`

## Available Tools

| Tool | Description |
|------|-------------|
| `search_text` | Semantic search across all tenders |
| `list_releases` | Filter and browse tenders by month, CPV code, category, value range |
| `get_release` | Full tender details by OCID |
| `get_index_info` | Database statistics and connectivity check |
| `create_company_profile` | Create a matching profile for your company |
| `update_company_profile` | Update an existing profile |
| `get_company_profile` | View profile details |
| `list_company_profiles` | List all your profiles |
| `delete_company_profile` | Delete a profile |
| `match_tenders` | Match a profile against all tenders with semantic similarity |

## How It Works

The npm package downloads the correct platform-native binary on install. No Node.js runtime dependency for the actual MCP server.

```
LLM <--stdio--> ocds-mcp (local binary)
                   |  Local: company profiles + sentence embedder
                   |  Remote: searches, release queries
                   +--HTTPS--> Vergabe Dashboard API
```

- Company profiles are stored locally (never leave your machine).
- Text embeddings are computed locally (multilingual-e5-small ONNX model, ~118 MB, auto-downloaded on first run).
- Only embedding vectors are sent to the API for search and matching.

## Supported Platforms

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64 |
| macOS | Apple Silicon (ARM64) |
| Windows | x86_64 |

## Requirements

- An API key from [vergabe-dashboard.qune.de](https://vergabe-dashboard.qune.de)
- ~200 MB disk space for the ONNX model (auto-downloaded on first run)
- Internet connection to reach the API

## License

MIT
