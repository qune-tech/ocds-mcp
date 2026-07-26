<!-- mcp-name: io.github.qune-tech/vergabe-mcp -->

# @qune-tech/vergabe-mcp

Local MCP server for German public procurement search. Connects your AI assistant to the [Vergabe Dashboard](https://vergabe-dashboard.qune.de) API for semantic search, tender matching, and company-profile management.

Your queries and company profiles never leave your machine — they are embedded locally and only the resulting vectors are sent. Data minimisation by design.

> **Hosted alternative:** if you just want search from your AI assistant, there is a hosted MCP connector (OAuth sign-in, no installation) — see the [setup guide](https://vergabe-dashboard.qune.de/ki-vergabe/einrichtung). This local server is for running the client in your own environment: queries and profiles are embedded locally, only vectors and filter values leave the machine, and it unlocks API-key features.

> New to German public procurement? The [Vergabe Dashboard knowledge base](https://vergabe-dashboard.qune.de/wissen/) explains eForms, EU thresholds, and the tender lifecycle, and [KI für Vergabe](https://vergabe-dashboard.qune.de/ki-vergabe/) covers the hosted AI side of this server.

## Quick Start

```bash
npx @qune-tech/vergabe-mcp --api-key sk_live_YOUR_KEY_HERE
```

### Get an API key

Sign up at [vergabe-dashboard.qune.de](https://vergabe-dashboard.qune.de) and create an API key. API keys require an active **Pro plan** (€99/month).

## Configure your AI client

### Claude Desktop

Edit `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "vergabe": {
      "command": "npx",
      "args": ["-y", "@qune-tech/vergabe-mcp", "--api-key", "sk_live_YOUR_KEY_HERE"]
    }
  }
}
```

### Claude Code

Add `.mcp.json` to your project root:

```json
{
  "mcpServers": {
    "vergabe": {
      "command": "npx",
      "args": ["-y", "@qune-tech/vergabe-mcp", "--api-key", "sk_live_YOUR_KEY_HERE"]
    }
  }
}
```

### Cursor

Settings > MCP Servers > Add:

- Command: `npx`
- Args: `-y @qune-tech/vergabe-mcp --api-key sk_live_YOUR_KEY_HERE`

### LM Studio

Settings > MCP > Add Server (STDIO):

- Name: `vergabe`
- Command: `npx`
- Arguments: `-y @qune-tech/vergabe-mcp --api-key sk_live_YOUR_KEY_HERE`

## Available Tools (11)

| Tool | Description |
|------|-------------|
| `search_text` | Semantic search across all tenders (query embedded locally) |
| `list_releases` | Filter and browse tenders by phase, CPV prefix, country, value range, deadline, buyer, procurement method |
| `get_release` | Raw [eForms](https://vergabe-dashboard.qune.de/wissen/eforms/) XML envelope for one OCID (optional `notice_id` selects a sibling) |
| `linked_notices` | A procurement's notice lineage (PIN→CN→CAN) as `{ocid, notice_id}` refs |
| `get_index_info` | API health/version, embedder status, and embedding-contract check |
| `create_company_profile` | Create a matching profile for your company (stored locally) |
| `update_company_profile` | Update an existing profile |
| `get_company_profile` | View profile details |
| `list_company_profiles` | List all your profiles |
| `delete_company_profile` | Delete a profile |
| `match_tenders` | Match a profile against all tenders by semantic similarity (vector embedded locally) |

## How It Works

The npm package downloads the correct platform-native binary on install. No Node.js runtime dependency for the actual MCP server.

```
LLM <--stdio--> vergabe-mcp (local binary)
                   |  Local: company profiles (SQLite) + sentence embedder (ONNX)
                   |  HTTPS: vectors, OCIDs, filter values, API key
                   +--HTTPS--> Vergabe Dashboard API (/api/v1)
```

- Company profiles are stored locally (never leave your machine).
- Text embeddings are computed locally (multilingual-e5-small ONNX model, ~118 MB, downloaded from huggingface.co on first run, cached afterwards).
- Only embedding vectors, the OCIDs you fetch, filter values, and your API key are sent to the API.

## Supported Platforms

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64 |
| macOS | Apple Silicon (ARM64) |
| Windows | x86_64 |

## Requirements

- An API key from [vergabe-dashboard.qune.de](https://vergabe-dashboard.qune.de) on the Pro plan (€99/month)
- ~120 MB disk space for the ONNX model (auto-downloaded on first run)
- Internet connection to reach the API

## License

MIT
