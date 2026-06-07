# vergabe-mcp

Local MCP server for German public procurement search. Connects your AI assistant (Claude, GPT, etc.) to the [Vergabe Dashboard](https://vergabe-dashboard.qune.de) API for semantic search, tender matching, and company-profile management.

**Your queries and company profiles never leave your machine.** They are embedded locally with a multilingual ONNX model; only the resulting embedding vectors, OCIDs, and filter values are sent to the API. Data minimisation by design.

> New to German public procurement? The [Vergabe Dashboard knowledge base](https://vergabe-dashboard.qune.de/wissen/) explains eForms, EU thresholds, and the tender lifecycle, and [KI für Vergabe](https://vergabe-dashboard.qune.de/ki-vergabe/) covers the hosted AI side of this server.

## Quick Start

### 1. Get an API key

Sign up at [vergabe-dashboard.qune.de](https://vergabe-dashboard.qune.de) and create an API key. API keys require an active **Enterprise plan** (the local server is free and open source; the API gate rides key issuance).

### 2. Install

**Via npx** (easiest — downloads the correct binary automatically):

```bash
npx @qune-tech/vergabe-mcp --api-key sk_live_YOUR_KEY_HERE
```

**Or download a pre-built binary** from [GitHub Releases](https://github.com/qune-tech/vergabe-mcp/releases/latest):

| Platform | Download |
|----------|----------|
| Linux x86_64 | [vergabe-mcp-linux-x86_64.tar.gz](https://github.com/qune-tech/vergabe-mcp/releases/latest/download/vergabe-mcp-linux-x86_64.tar.gz) |
| macOS Apple Silicon | [vergabe-mcp-macos-arm64.tar.gz](https://github.com/qune-tech/vergabe-mcp/releases/latest/download/vergabe-mcp-macos-arm64.tar.gz) |
| Windows x86_64 | [vergabe-mcp-windows-x86_64.zip](https://github.com/qune-tech/vergabe-mcp/releases/latest/download/vergabe-mcp-windows-x86_64.zip) |

**Linux / macOS:**

```bash
# Example for Linux x86_64 — adjust the filename for your platform
tar xzf vergabe-mcp-linux-x86_64.tar.gz
sudo mv vergabe-mcp /usr/local/bin/vergabe-mcp
```

**Windows:** Extract the zip and move `vergabe-mcp.exe` somewhere on your PATH.

**Or build from source:**

```bash
git clone https://github.com/qune-tech/vergabe-mcp.git
cd vergabe-mcp
cargo build --release
# Binary at target/release/vergabe-mcp
```

### 3. Configure your AI client

**Claude Desktop** — edit `claude_desktop_config.json`:

Using npx:
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

Using the binary directly:
```json
{
  "mcpServers": {
    "vergabe": {
      "command": "vergabe-mcp",
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
    "vergabe": {
      "command": "npx",
      "args": ["-y", "@qune-tech/vergabe-mcp", "--api-key", "sk_live_YOUR_KEY_HERE"]
    }
  }
}
```

Using the binary directly:
```json
{
  "mcpServers": {
    "vergabe": {
      "command": "vergabe-mcp",
      "args": ["--api-key", "sk_live_YOUR_KEY_HERE"]
    }
  }
}
```

**Cursor** — Settings → MCP Servers → Add:

Using npx:
- Command: `npx`
- Args: `-y @qune-tech/vergabe-mcp --api-key sk_live_YOUR_KEY_HERE`

Using the binary directly:
- Command: `vergabe-mcp`
- Args: `--api-key sk_live_YOUR_KEY_HERE`

**LM Studio** — Settings → MCP → Add Server:

1. Click **+ Add Server** and choose **STDIO**
2. Fill in:

Using npx:
   - Name: `vergabe`
   - Command: `npx`
   - Arguments: `-y @qune-tech/vergabe-mcp --api-key sk_live_YOUR_KEY_HERE`

Using the binary directly:
   - Name: `vergabe`
   - Command: full path to the binary, e.g. `/usr/local/bin/vergabe-mcp`
   - Arguments: `--api-key sk_live_YOUR_KEY_HERE`

3. Click **Save**
4. In the chat, select a model that supports tool use and enable the `vergabe` server

> LM Studio requires models with tool-calling support (e.g. Qwen 2.5, Mistral, Llama 3.1+).
> Smaller models may not use all 11 tools reliably — 7B+ recommended.

Replace `sk_live_YOUR_KEY_HERE` with your actual API key. You can also pass the key via the `VERGABE_API_KEY` environment variable.

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

## CLI Options

```
Usage: vergabe-mcp [OPTIONS]

Options:
      --db <DB>            Local profiles database [default: profiles.db]
      --data-dir <DIR>     Data directory [default: data]
      --api-url <URL>      Vergabe Dashboard API [default: https://vergabe-dashboard.qune.de]
      --api-key <KEY>      API key [env: VERGABE_API_KEY]
  -h, --help               Print help
```

## How It Works

```
LLM ←stdio→ vergabe-mcp (local)
               │  Local: company profiles (SQLite) + sentence embedder (ONNX)
               │  HTTPS: vectors, OCIDs, filter values, API key
               └──HTTPS──→ Vergabe Dashboard API (/api/v1)
```

The MCP server runs locally on your machine:

- **Company profiles** (name, description, CPV interests, location) are stored in a local SQLite database — they never leave your network.
- **Text embeddings** are computed locally with a multilingual ONNX model (multilingual-e5-small, 384-dim). Search queries use the `query:` prefix and go to `POST /api/v1/search/vector`; profile descriptions use the `passage:` prefix and go to `POST /api/v1/match/vector`.
- Only **embedding vectors** (384 floats), the **OCIDs** you fetch, **filter values**, and your **API key** are sent to the API. Your query and profile text stay local.

## Privacy & data flow

What stays on your machine: profile text, search-query text, CPV interests, location.

What the API sees: embedding vectors, the OCIDs you read, the filter values you use, and your API key. These reveal *commercial interest* (which sectors, buyers, value bands, tenders you look at) but not the underlying text. Embedding vectors are a derived, pseudonymous representation — minimisation, not elimination.

**First-run model download:** on first use the server downloads the embedding model from **huggingface.co**:

- **What:** `model.onnx` + `tokenizer.json`, ~118 MB total.
- **When:** the first time the embedder runs; cached afterwards at `~/.cache/vergabe/models/multilingual-e5-small`.
- **From:** `huggingface.co` (a US-operated third party). **No user data** is sent in this fetch — it is a plain model download.

For air-gapped / enterprise installs, place `model.onnx` and `tokenizer.json` in the cache directory above and the runtime download is skipped.

## Requirements

- An API key from [vergabe-dashboard.qune.de](https://vergabe-dashboard.qune.de) on an Enterprise plan
- ~120 MB disk space for the ONNX model (downloaded automatically on first run)
- Internet connection to reach the API

## License

MIT — see [LICENSE](LICENSE).
