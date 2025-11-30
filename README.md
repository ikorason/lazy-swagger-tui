# lazy-swagger-tui

A fast terminal UI for testing APIs directly from Swagger/OpenAPI specs.

## Features

- 🔍 Auto-discover endpoints from Swagger/OpenAPI
- ⚡ Fast keyboard-driven workflow
- 🔐 Bearer token authentication
- 🔎 Live search and filtering
- 📝 JSON body editor with auto-formatting
- 🌐 All HTTP methods (GET, POST, PUT, PATCH, DELETE)
- 🎨 Adapts to your terminal theme

## Installation

### From crates.io

```bash
cargo install lazy-swagger-tui
```

### From GitHub Releases

Download pre-built binaries from [releases](https://github.com/ikorason/lazy-swagger-tui/releases).

### From source

```bash
git clone https://github.com/ikorason/lazy-swagger-tui
cd lazy-swagger-tui
cargo install --path .
```

## Quick Start

```bash
lazy-swagger-tui
```

On first launch, configure:
- **Swagger URL**: `http://localhost:5000/swagger/v1/swagger.json`
- **Base URL**: `http://localhost:5000`

Then navigate with `j`/`k` and press `Space` to execute requests.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j`/`k` or `↑`/`↓` | Navigate endpoints |
| `Space` | Execute request |
| `/` | Search/filter |
| `Tab` / `Shift+Tab` | Switch panels/tabs |
| `e` | Edit parameter |
| `b` | Edit JSON body (POST/PUT/PATCH) |
| `a` | Set auth token |
| `g` | Toggle grouped/flat view |
| `1` / `2` | Jump to panel |
| `q` | Quit |

## Search

Press `/` and start typing to filter endpoints by path, method, summary, or tags. Press `Esc` to clear.

## Configuration

Config is stored in `~/.config/lazy-swagger-tui/config.toml`:

```toml
[server]
swagger_url = "http://localhost:5000/swagger/v1/swagger.json"
base_url = "http://localhost:5000"
```

Press `,` in the app to update URLs.

## License

MIT
