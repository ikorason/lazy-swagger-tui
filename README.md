# dotREST

A Ratatui-based terminal UI for testing ASP.NET Core APIs directly from your terminal.

## Features

- 🔍 Automatically discovers endpoints from Swagger/OpenAPI specifications
- 🎯 Efficient terminal UI for browsing and testing endpoints
- 🔐 Bearer token authentication support
- 📊 Grouped endpoint views by tags
- ⚡ Fast, keyboard-driven workflow

## Quick Start

1. Run the application:

```bash
   cargo run
```

2. On first launch, you'll be prompted to configure:
   - **Swagger URL**: The URL to your OpenAPI/Swagger specification
     - Example: `http://localhost:5000/swagger/v1/swagger.json`
   - **API Base URL**: The base URL for making API requests (auto-detected)
     - Example: `http://localhost:5000`

3. Navigate endpoints with arrow keys, press `Enter` to execute requests

## Authentication

dotREST supports Bearer token authentication:

- Press `a` to set/edit your authentication token
- Press `A` (Shift+A) to clear the token
- Tokens are stored in memory only (not persisted to disk)
- The token is automatically included in all API requests as: `Authorization: Bearer <your-token>`

**Getting a Token:**
Most APIs require you to authenticate first to get a token:

1. Use Swagger UI or another tool to call your login endpoint
2. Copy the token from the response
3. Press `a` in dotREST and paste your token
4. The token will be included in all subsequent requests

**Token Security:**

- Tokens are displayed in masked format: `abc1234...xyz789`
- Only first 7 and last 6 characters are shown
- Tokens are cleared when the application exits

## Configuration

Configuration is stored in `~/.config/dotrest/config.toml`:

```toml
[server]
swagger_url = "http://localhost:5000/swagger/v1/swagger.json"
base_url = "http://localhost:5000"
```

You can edit this file directly or press `u` in the app to update URLs.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑↓` | Navigate endpoints |
| `Enter` | Execute request (or expand/collapse groups) |
| `g` / `G` | Toggle grouped/flat view |
| `u` / `U` | Configure URLs |
| `a` | Set/edit authentication token |
| `A` | Clear authentication token |
| `F5` | Refresh endpoints |
| `r` / `R` | Retry after error |
| `q` | Quit |

## Building from Source

```bash
cargo build --release
```

The binary will be available at `target/release/dotrest`.
