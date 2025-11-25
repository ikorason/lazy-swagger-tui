# lazy swagger tui

A Ratatui-based terminal UI for testing ASP.NET Core APIs directly from your terminal.

## Features

- 🔍 Automatically discovers endpoints from Swagger/OpenAPI specifications
- 🎯 Efficient terminal UI for browsing and testing endpoints
- 🔐 Bearer token authentication support
- 📊 Grouped endpoint views by tags
- 📑 Tab-based details panel (Endpoint, Headers, Response)
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

3. Navigate endpoints with arrow keys or `j`/`k`, press `Space` to execute requests

## Authentication

lazy swagger tui supports Bearer token authentication:

- Press `a` to set/edit your authentication token
- Press `A` (Shift+A) to clear the token
- Tokens are stored in memory only (not persisted to disk)
- The token is automatically included in all API requests as: `Authorization: Bearer <your-token>`

**Getting a Token:**
Most APIs require you to authenticate first to get a token:

1. Use Swagger UI or another tool to call your login endpoint
2. Copy the token from the response
3. Press `a` in lazy swagger tui and paste your token
4. The token will be included in all subsequent requests

## Configuration

Configuration is stored in `~/.config/lazy-swagger-tui/config.toml`:

```toml
[server]
swagger_url = "http://localhost:5000/swagger/v1/swagger.json"
base_url = "http://localhost:5000"
```

You can edit this file directly or press `,` in the app to update URLs.

## User Interface

### Two-Panel Layout

- **Left Panel**: Endpoints list (grouped by tags or flat view)
- **Right Panel**: Details with three tabs
  - **Endpoint Tab**: Shows method, path, summary, and tags
  - **Headers Tab**: Displays response headers
  - **Response Tab**: Shows response body, status, and duration

### Navigation Flow

The UI uses a consistent left-to-right navigation model:

```
Endpoints Panel → Endpoint Tab → Headers Tab → Response Tab → (wraps back to Endpoints)
```

- Press `Tab` to move right through panels and tabs
- Press `Shift+Tab` to move left through panels and tabs
- Active panel and tab are highlighted in cyan

## Keyboard Shortcuts

### Global Commands

| Key | Action |
|-----|--------|
| `Tab` | Move right (panel → panel, tab → tab) |
| `Shift+Tab` | Move left (panel → panel, tab → tab) |
| `,` | Configure URLs |
| `a` | Set/edit authentication token |
| `r` | Retry after error |
| `g` | Toggle grouped/flat view |
| `q` | Quit |

### Endpoints Panel (Left Side)

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down through endpoints |
| `↑` / `↓` | Navigate up/down (alternative) |
| `Ctrl+d` / `Ctrl+u` | Scroll half-page down/up in detail panels |
| `Space` | Execute selected endpoint or toggle group |

### Details Panel (Right Side)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between tabs |
| `Space` | Execute currently selected endpoint |
| `Ctrl+d` / `Ctrl+u` | Scroll content down/up in active tab |

**Note**: Line-by-line scrolling with `j`/`k` in the Details panel is coming in a future update.

## Tips & Tricks

- **Quick Testing**: Select an endpoint and press `Space` to execute. Press `Space` again in the Details panel to re-execute.
- **Compare Responses**: Switch between Headers and Response tabs to inspect different aspects of the API response.
- **Paste Support**: When entering tokens or URLs, you can paste large amounts of text - the app handles it efficiently.
- **Grouped Navigation**: In grouped view, press `Space` on a group header to expand/collapse it.

## Building from Source

```bash
cargo build --release
```

The binary will be available at `target/release/lazy-swagger-tui`.

## Roadmap

- [ ] Line-by-line scrolling with `j`/`k` in Details panel
- [ ] Support for POST/PUT/PATCH requests with JSON body editing
- [ ] Request history and favorites
- [ ] Environment variable support
- [ ] Export responses to files
- [ ] JSON syntax highlighting
