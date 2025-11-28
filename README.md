# lazy swagger tui

A Ratatui-based terminal UI for testing ASP.NET Core APIs directly from your terminal.

## Features

- 🔍 Automatically discovers endpoints from Swagger/OpenAPI specifications
- 🎯 Efficient terminal UI for browsing and testing endpoints
- 🔐 Bearer token authentication support
- 🔎 **Live search** - Filter endpoints by path, method, summary, or tags
- 📊 Grouped endpoint views by tags
- 📑 Tab-based details panel (Endpoint, Request, Headers, Response)
- 🌐 **Full HTTP method support** - GET, POST, PUT, PATCH, DELETE
- 📝 **JSON body editing** - Multi-line editor with auto-formatting for POST/PUT/PATCH requests
- ⚡ Fast, keyboard-driven workflow

## Quick Start

1. Run the application:

```bash
   cargo run
```

2. On first launch, you'll be prompted to configure:
   - **Swagger URL**: The URL to your OpenAPI/Swagger specification (for fetching endpoints)
     - Example: `http://localhost:5000/swagger/v1/swagger.json`
   - **API Base URL**: The base URL for making API requests (must be entered manually)
     - Example: `http://localhost:5000`
   - Use `Tab` to switch between fields, `Ctrl+L` to clear the current field

3. Navigate endpoints with arrow keys or `j`/`k`, press `Space` to execute requests

## Search & Filter

Quickly find endpoints with live search:

1. Press **`/`** to activate search
2. Type to filter endpoints in real-time
3. Press **`Esc`** or **`Enter`** to exit search mode (filter stays active)
4. Press **`Ctrl+L`** to clear the filter

**Search matches against:**

- Endpoint paths (e.g., `/api/users`)
- HTTP methods (GET, POST, etc.)
- Endpoint summaries/descriptions
- Tags/groups

The search is **case-insensitive** and shows live results with a match counter (e.g., `[5/25]`).

## Authentication

lazy swagger tui supports Bearer token authentication:

- Press `a` to set/edit your authentication token
- Press `A` (Shift+A) to clear the token
- Tokens are stored in memory only (not persisted to disk)
- The token is automatically included in all API requests as: `Authorization: Bearer <your-token>`

## Configuration

Configuration is stored in `~/.config/lazy-swagger-tui/config.toml`:

```toml
[server]
swagger_url = "http://localhost:5000/swagger/v1/swagger.json"
base_url = "http://localhost:5000"
```

**Important**: Both URLs are required and independent:

- **Swagger URL**: Used to fetch the list of available endpoints from your API documentation
- **Base URL**: Used as the base for all actual API requests

You can edit this file directly or press `,` in the app to update URLs.

### URL Configuration Modal

When configuring URLs (press `,`):

- Use `Tab` to switch between Swagger URL and Base URL fields
- Use `Ctrl+L` to clear the current field
- Use `Ctrl+W` to delete the previous word
- Press `Enter` to confirm, `Esc` to cancel

## User Interface

### Layout

- **Search Bar**: Live filter for finding endpoints (press `/`)
- **Left Panel**: Endpoints list (grouped by tags or flat view)
- **Right Panel**: Details with four tabs
  - **Endpoint Tab**: Shows method, path, summary, and tags
  - **Request Tab**: Configure path/query parameters and JSON body (press `e` to edit params, `b` for body)
  - **Headers Tab**: Displays response headers
  - **Response Tab**: Shows response body, status, and duration

### Navigation Flow

The UI uses a consistent left-to-right navigation model:

```
Endpoints Panel → Endpoint Tab → Request Tab → Headers Tab → Response Tab → (wraps back to Endpoints)
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
| `1` | **Focus Endpoints panel (left)** |
| `2` | **Focus Details panel (right)** |
| `/` | **Search/filter endpoints** |
| `Ctrl+L` | Clear search filter |
| `,` | Configure URLs |
| `a` | Set/edit authentication token |
| `g` | Toggle grouped/flat view |
| `q` | Quit |

### Endpoints Panel (Left Side)

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate up/down through endpoints |
| `↑` / `↓` | Navigate up/down (alternative) |
| `Space` | Execute selected endpoint or toggle group |

### Search Mode

| Key | Action |
|-----|--------|
| Type | Filter endpoints in real-time |
| `Backspace` | Delete last character |
| `Ctrl+L` | Clear entire search query |
| `Enter` | Exit search mode (keeps filter active) |
| `Esc` | Exit search mode and clear filter |

**Search matches**: path, method, summary, and tags (case-insensitive)

### Details Panel (Right Side)

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Switch between tabs |
| `Space` | Execute currently selected endpoint |
| `j` / `k` | Navigate parameters (Request tab only) |
| `e` | Edit selected parameter (Request tab only) |
| `b` | Edit JSON body (Request tab, POST/PUT/PATCH only) |
| `x` | Toggle body section collapse/expand (Request tab only) |
| `Enter` | Confirm edit (parameter or body) |
| `Esc` | Cancel edit |
| `Ctrl+L` | Clear body (while editing body) |

## Tips & Tricks

- **Quick Testing**: Select an endpoint and press `Space` to execute. Press `Space` again in the Details panel to re-execute.
- **Panel Switching**: Press `1` to jump to Endpoints panel, `2` to jump to Details panel. Or use `Tab`/`Shift+Tab` to cycle through.
- **Search**: Press `/` and start typing to filter endpoints. Press `Enter` to exit and keep the filter active, or `Esc` to exit and clear the filter. You can also clear the filter anytime with `Ctrl+L`.
- **Parameters**: Navigate to the Request tab to configure path and query parameters before executing endpoints. Press `e` to edit, `Enter` to confirm.
- **JSON Body Editing**: For POST/PUT/PATCH endpoints, press `b` in the Request tab to open the body editor. Type or paste your JSON, press `Enter` to auto-format and save. Invalid JSON is kept as-is without errors.
- **Body Preview**: The Request tab shows a collapsible body preview (first 5 lines). Press `x` to toggle collapse/expand.
- **Parameter-less Endpoints**: Endpoints without path parameters can be executed immediately with `Space` - no configuration needed!
- **Compare Responses**: Switch between Headers and Response tabs to inspect different aspects of the API response.
- **Paste Support**: When entering tokens, URLs, parameters, or JSON bodies, you can paste large amounts of text - the app handles it efficiently.
- **Grouped Navigation**: In grouped view, press `Space` on a group header to expand/collapse it.
- **URL Configuration**: The Swagger URL and Base URL are completely independent - set them both correctly for your API.

## Building from Source

```bash
cargo build --release
```

The binary will be available at `target/release/lazy-swagger-tui`.

## Roadmap

- [x] JSON body editing for POST/PUT/PATCH requests
- [ ] Parse Swagger requestBody schemas with `$ref` resolution
- [ ] Generate example values from schema (like Swagger UI)
- [ ] Request history and favorites
- [ ] Environment variable support
- [ ] Export responses to files
- [ ] JSON syntax highlighting in body editor
