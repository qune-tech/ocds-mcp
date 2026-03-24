# ocds-mcp

MCP server for German public procurement data (OCDS). Connects your AI assistant (Claude, GPT, etc.) to the [Vergabe Dashboard](https://vergabe-dashboard.qune.de) API for semantic search, tender matching, and company profile management.

Your company profiles never leave your machine — only embedding vectors are sent to the API. GDPR-compliant by design.

## Quick Start

### 1. Get an API key

Sign up at [vergabe-dashboard.qune.de](https://vergabe-dashboard.qune.de) and create an API key (MCP or Enterprise plan required).

### 2. Install

**Via npx** (easiest — downloads the correct binary automatically):

```bash
npx @qune-tech/ocds-mcp --api-key sk_live_YOUR_KEY_HERE
```

**Or download pre-built binary** from [GitHub Releases](https://github.com/qune-tech/ocds-mcp/releases/latest):

| Platform | Download |
|----------|----------|
| Linux x86_64 | [ocds-mcp-linux-x86_64.tar.gz](https://github.com/qune-tech/ocds-mcp/releases/latest/download/ocds-mcp-linux-x86_64.tar.gz) |
| macOS Apple Silicon | [ocds-mcp-macos-arm64.tar.gz](https://github.com/qune-tech/ocds-mcp/releases/latest/download/ocds-mcp-macos-arm64.tar.gz) |
| Windows x86_64 | [ocds-mcp-windows-x86_64.zip](https://github.com/qune-tech/ocds-mcp/releases/latest/download/ocds-mcp-windows-x86_64.zip) |

**Linux / macOS:**

```bash
# Example for Linux x86_64 — adjust the filename for your platform
tar xzf ocds-mcp-linux-x86_64.tar.gz
sudo mv ocds-mcp-linux-x86_64 /usr/local/bin/ocds-mcp
```

**Windows:** Extract the zip and move `ocds-mcp-windows-x86_64.exe` somewhere on your PATH (e.g. `C:\Users\YOU\.local\bin\ocds-mcp.exe`).

**Or build from source:**

```bash
git clone https://github.com/qune-tech/ocds-mcp.git
cd ocds-mcp
cargo build --release
# Binary at target/release/ocds-mcp
```

### 3. Configure your AI client

**Claude Desktop** — edit `claude_desktop_config.json`:

Using npx:
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

Using the binary directly:
```json
{
  "mcpServers": {
    "ocds": {
      "command": "ocds-mcp",
      "args": ["--api-key", "sk_live_YOUR_KEY_HERE"]
    }
  }
}
```

**Claude Code** — add `.mcp.json` to your project root:

Using npx:
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

Using the binary directly:
```json
{
  "mcpServers": {
    "ocds": {
      "command": "ocds-mcp",
      "args": ["--api-key", "sk_live_YOUR_KEY_HERE"]
    }
  }
}
```

**Cursor** — Settings → MCP Servers → Add:

Using npx:
- Command: `npx`
- Args: `-y @qune-tech/ocds-mcp --api-key sk_live_YOUR_KEY_HERE`

Using the binary directly:
- Command: `ocds-mcp`
- Args: `--api-key sk_live_YOUR_KEY_HERE`

**LM Studio** — Settings → MCP → Add Server:

1. Click **+ Add Server** and choose **STDIO**
2. Fill in:

Using npx:
   - Name: `ocds`
   - Command: `npx`
   - Arguments: `-y @qune-tech/ocds-mcp --api-key sk_live_YOUR_KEY_HERE`

Using the binary directly:
   - Name: `ocds`
   - Command: full path to the binary, e.g. `/usr/local/bin/ocds-mcp`
   - Arguments: `--api-key sk_live_YOUR_KEY_HERE`

3. Click **Save**
4. In the chat, select a model that supports tool use and enable the `ocds` server

> LM Studio requires models with tool-calling support (e.g. Qwen 2.5, Mistral, Llama 3.1+).
> Smaller models may not use all 10 tools reliably — 7B+ recommended.

Replace `sk_live_YOUR_KEY_HERE` with your actual API key.

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

## CLI Options

```
Usage: ocds-mcp [OPTIONS]

Options:
      --db <DB>            Local profiles database [default: profiles.db]
      --data-dir <DIR>     Data directory [default: data]
      --api-url <URL>      Vergabe Dashboard API [default: https://vergabe-dashboard.qune.de]
      --api-key <KEY>      API key [env: OCDS_API_KEY]
  -h, --help               Print help
```

## How It Works

```
LLM ←stdio→ ocds-mcp (local)
               │  Local: company profiles + sentence embedder
               │  Remote: searches, release queries
               └──HTTPS──→ Vergabe Dashboard API
```

The MCP server runs locally on your machine:

- **Company profiles** are stored in a local SQLite database — they never leave your network.
- **Text embeddings** are computed locally using a multilingual ONNX model (multilingual-e5-small, ~118 MB, auto-downloaded on first use).
- Only **embedding vectors** (arrays of 384 floats) are sent to the API for search and matching — your profile text stays local.
- Tender data is fetched from the API on demand.

## Requirements

- An API key from [vergabe-dashboard.qune.de](https://vergabe-dashboard.qune.de) (MCP or Enterprise plan)
- ~200 MB disk space for the ONNX model (downloaded automatically on first run)
- Internet connection to reach the API

## License

MIT — see [LICENSE](LICENSE).
