# Contributing to Cloudflare Backup

Thank you for your interest in contributing! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- **Node.js** 20+ and npm
- **Rust** (latest stable)
- **System dependencies** (for Tauri):
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Visual Studio Build Tools, WebView2
  - **Linux**: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev`

### Getting Started

1. **Clone the repository**
   ```bash
   git clone https://github.com/joshfom/tauri-drive.git
   cd tauri-drive
   ```

2. **Install dependencies**
   ```bash
   npm install
   ```

3. **Run in development mode**
   ```bash
   npm run tauri dev
   ```

## Running Tests

Before submitting a PR, ensure all tests pass:

### TypeScript Tests
```bash
npm test
```

### Rust Tests
```bash
cd src-tauri
cargo test
```

### Full Test Suite
```bash
npm test && cd src-tauri && cargo test
```

## Pull Request Process

1. **Create a feature branch**
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** and ensure:
   - All tests pass
   - Code follows existing style
   - New features have tests

3. **Commit with descriptive messages**
   ```bash
   git commit -m "feat: add new feature description"
   ```
   
   We follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` - New features
   - `fix:` - Bug fixes
   - `docs:` - Documentation changes
   - `test:` - Adding/updating tests
   - `refactor:` - Code refactoring
   - `chore:` - Maintenance tasks

4. **Push and create a PR**
   ```bash
   git push origin feature/your-feature-name
   ```
   Then create a Pull Request on GitHub.

5. **Wait for CI checks**
   - TypeScript tests must pass
   - Rust tests must pass
   - Build check must succeed

## Project Structure

```
├── src/                    # Svelte frontend
│   ├── components/         # UI components
│   ├── lib/               
│   │   ├── stores/        # Svelte stores
│   │   ├── types/         # TypeScript types
│   │   └── utils/         # Utility functions
│   └── routes/            # Page components
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── crypto/        # Encryption module
│   │   ├── db/            # SQLite database
│   │   ├── migration/     # Backup/restore
│   │   ├── r2/            # Cloudflare R2 client
│   │   ├── upload/        # Upload manager
│   │   └── utils/         # Shared types
│   └── migrations/        # SQL migrations
└── static/                # Static assets
```

## Code Style

### TypeScript/Svelte
- Use TypeScript for type safety
- Follow existing formatting (Prettier)
- Use Svelte 5 runes (`$state`, `$derived`, etc.)

### Rust
- Follow standard Rust conventions
- Run `cargo clippy` before committing
- Use `anyhow::Result` for error handling

## Need Help?

- Open an issue for bugs or feature requests
- Check existing issues before creating new ones
- Be respectful and constructive in discussions

Thank you for contributing! 🎉
