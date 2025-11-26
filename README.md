# Tauri R2 Drive

A modern, cross-platform desktop application for Cloudflare R2 storage built with Tauri, Svelte, and Rust.

## ✨ Features

### Current (MVP)
- 🔐 **R2 Connection** - Connect to Cloudflare R2 with your credentials
- 📁 **File Browser** - Browse your R2 bucket contents
- ⬆️ **Upload/Download** - Transfer files to and from R2
- 📊 **Multipart Uploads** - Automatically handles large files (300GB+) with chunking
- 🎨 **Modern UI** - Clean interface with dark mode support
- ⚡ **Fast & Lightweight** - Built with Rust and Svelte for optimal performance

### Coming Soon
- ⏸️ **Resume Uploads** - Resume interrupted uploads from where they left off
- 🔄 **Folder Sync** - Two-way synchronization with local folders
- 📈 **Transfer Queue** - Manage multiple uploads/downloads with priority
- 🖼️ **File Preview** - Preview images, videos, and documents
- 📊 **Bandwidth Control** - Throttle upload/download speeds
- 🔔 **System Tray** - Run in background with system tray integration

## 🚀 Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (v18 or later)
- [Rust](https://rustup.rs/) (latest stable)
- [Tauri CLI](https://tauri.app/start/prerequisites/)

### Installation

1. **Clone the repository**
   ```bash
   git clone <your-repo-url>
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

4. **Build for production**
   ```bash
   npm run tauri build
   ```

## 🔧 Configuration

### R2 Credentials

1. Go to [Cloudflare Dashboard](https://dash.cloudflare.com) → R2
2. Create or select a bucket
3. Navigate to "Manage R2 API Tokens"
4. Create a new API token with read/write permissions
5. Copy your:
   - Account ID
   - Access Key ID
   - Secret Access Key
   - Bucket Name

6. Open the app and navigate to **Settings**
7. Enter your credentials and click **Connect to R2**

## 📁 Project Structure

```
tauri-drive/
├── src/                      # Frontend (Svelte)
│   ├── routes/              # Page components
│   │   ├── Browser.svelte   # File browser
│   │   ├── Settings.svelte  # R2 configuration
│   │   └── Transfers.svelte # Transfer queue
│   ├── lib/
│   │   ├── types/           # TypeScript types
│   │   ├── stores/          # Svelte stores
│   │   └── utils/           # Utilities
│   └── App.svelte           # Main app component
│
├── src-tauri/               # Backend (Rust)
│   ├── src/
│   │   ├── r2/              # R2 client module
│   │   │   ├── client.rs    # S3-compatible client
│   │   │   ├── operations.rs # CRUD operations
│   │   │   └── multipart.rs  # Multipart uploads
│   │   ├── db/              # SQLite database
│   │   ├── upload/          # Upload engine (coming soon)
│   │   ├── sync/            # Sync engine (coming soon)
│   │   └── utils/           # Shared types & utilities
│   └── migrations/          # Database schema
│
└── SPECS.md                 # Technical specifications
```

## 🛠️ Tech Stack

### Frontend
- **Framework**: Svelte 5 with TypeScript
- **Styling**: TailwindCSS
- **Routing**: svelte-spa-router
- **State**: Svelte stores (built-in)

### Backend (Rust)
- **Framework**: Tauri 2.x
- **AWS SDK**: aws-sdk-s3 (S3-compatible for R2)
- **Database**: SQLite with sqlx
- **Async**: Tokio
- **File Watching**: notify (for sync engine)

## 📊 Features Deep Dive

### Multipart Upload System
- Automatically chunks files > 100MB into 10MB parts
- Parallel upload workers (configurable)
- ETag verification for data integrity
- SQLite-based state tracking for resume capability

### Database Schema
- **buckets**: R2 account configurations
- **uploads**: Track upload progress and state
- **upload_chunks**: Individual chunk status for resume
- **sync_folders**: Sync configuration
- **file_metadata**: Cached file information

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📝 License

MIT

## 🙏 Acknowledgments

- Built with [Tauri](https://tauri.app/)
- Inspired by [Rclone](https://rclone.org/)
- Uses [Cloudflare R2](https://www.cloudflare.com/products/r2/)

---

**Note**: This is currently in active development. Features are being added regularly.
