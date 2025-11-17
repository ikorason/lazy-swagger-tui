# dotREST

A Ratatui-based terminal UI for testing ASP.NET Core APIs directly from your terminal.

> REST testing for .NET developers

## Project Structure

```
dotrest/
├── Cargo.toml          # Dependencies configured
└── src/
    └── main.rs         # Basic TUI skeleton with TODOs
```

# Configuration

## Swagger URL Setup

### First Launch

- Enter your Swagger/OpenAPI URL when prompted
- Example: `http://localhost:5000/swagger/v1/swagger.json`
- Press **Enter** to save

### Change URL Later

- Press **`u`** to update the URL anytime
- The new URL is saved automatically

### Config File

Location: `~/.config/dotrest/config.toml`

```toml
[server]
swagger_url = "http://localhost:5000/swagger/v1/swagger.json"
```

### URL Requirements

- Must start with `http://` or `https://`
- Must point to a valid Swagger/OpenAPI JSON specification
