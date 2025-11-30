# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-11-30

### Initial Release

#### Features
- 🔍 Automatic endpoint discovery from Swagger/OpenAPI specifications
- 🎯 Efficient keyboard-driven terminal UI
- 🔐 Bearer token authentication support
- 🔎 Live search and filtering by path, method, summary, or tags
- 📊 Grouped endpoint views by tags (toggle with `g`)
- 📑 Tab-based details panel (Endpoint, Request, Headers, Response)
- 🌐 Full HTTP method support (GET, POST, PUT, PATCH, DELETE)
- 📝 Multi-line JSON body editor with auto-formatting
- 📋 Clipboard integration for yanking response bodies
- ⚡ Fast paste support for tokens, URLs, and JSON
- 🎨 Respects terminal color themes (works in light and dark themes)

#### Keyboard Shortcuts
- Navigation: `j`/`k`, `↑`/`↓`, `Tab`, `Shift+Tab`
- Search: `/` to search, `Ctrl+L` to clear
- Execution: `Space` to execute requests
- Panel switching: `1` (endpoints), `2` (details)
- Authentication: `a` to set token, `A` to clear
- Parameter editing: `e` to edit, `Enter` to confirm
- Body editing: `b` to open editor (POST/PUT/PATCH)
- View toggle: `g` for grouped/flat view
- Quit: `q`

#### Configuration
- Config stored in `~/.config/lazy-swagger-tui/config.toml`
- Supports Swagger URL and Base URL configuration
- In-memory token storage (not persisted)

#### Technical
- Built with Ratatui for terminal UI
- Async HTTP requests with Tokio
- JSON parsing and validation
- Comprehensive test coverage (89 tests)

[Unreleased]: https://github.com/ikorason/lazy-swagger-tui/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ikorason/lazy-swagger-tui/releases/tag/v0.1.0
