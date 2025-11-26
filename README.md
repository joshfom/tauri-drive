# Cloudflare Backup

A beautiful, fast desktop app for backing up your files to Cloudflare R2 storage. Built with Tauri, Svelte 5, and Rust.

## ✨ Features

- 🚀 **Fast Uploads** - Concurrent multipart uploads with progress tracking
- 🔒 **Client-Side Encryption** - Optional AES-256-GCM encryption before upload
- 📁 **Folder Sync** - Automatically sync local folders to R2
- 📦 **Folder Downloads** - Download entire folders as ZIP archives
- 🎨 **Modern UI** - Clean Google Drive-inspired interface
- 💾 **Export/Import** - Backup and restore your configuration
- 🖥️ **Cross-Platform** - Works on macOS, Windows, and Linux

## 📥 Download

Download the latest release for your platform from the **[GitHub Releases](../../releases)** page:

| Platform | Download |
|----------|----------|
| **macOS** | `.dmg` installer |
| **Windows** | `.exe` or `.msi` installer |
| **Linux** | `.AppImage` or `.deb` package |

## 🔧 Setup

### Step 1: Create a Cloudflare R2 Bucket

1. Log in to [Cloudflare Dashboard](https://dash.cloudflare.com/)
2. Go to **R2 Object Storage** in the sidebar
3. Click **Create bucket**
4. Give your bucket a name (e.g., `my-backups`)
5. Choose a location and click **Create bucket**

### Step 2: Get Your Cloudflare R2 API Keys

You'll need three values to connect the app to your R2 bucket:

#### 📋 Account ID
1. In Cloudflare Dashboard, look at the URL: `https://dash.cloudflare.com/ACCOUNT_ID/...`
2. Or go to **R2** → **Overview** → Your Account ID is shown on the right side

#### 🔑 Access Key ID & Secret Access Key
1. Go to **R2** → **Overview**
2. Click **Manage R2 API Tokens** on the right side
3. Click **Create API token**
4. Give it a name (e.g., "Cloudflare Backup App")
5. Under **Permissions**, select:
   - **Object Read & Write** (to upload/download files)
6. Under **Specify bucket(s)**, either:
   - Select **Apply to all buckets**, or
   - Select your specific bucket
7. Click **Create API Token**
8. ⚠️ **Important**: Copy and save both values immediately:
   - **Access Key ID** (starts with a long string)
   - **Secret Access Key** (only shown once!)

### Step 3: Configure the App

1. Open Cloudflare Backup
2. Go to **Settings** (gear icon)
3. Enter your credentials:
   - **Account ID**: Your Cloudflare account ID
   - **Access Key ID**: From the API token you created
   - **Secret Access Key**: From the API token you created
   - **Bucket Name**: The name of your R2 bucket
4. Click **Test Connection** to verify
5. Click **Save**

## 🚀 Usage

### Uploading Files
- Click **Upload** or drag and drop files into the browser
- Files are uploaded with progress tracking
- Large files are automatically split for multipart upload

### Encryption
- Enable encryption in **Settings** to encrypt files before upload
- Files are encrypted with AES-256-GCM
- Encryption key is derived from your secret access key

### Sync Folders
1. Go to **Settings** → **Sync Folders**
2. Add folders you want to automatically sync
3. The app will keep your folders in sync with R2

### Download Files
- Click on a file to download it
- Right-click a folder and select "Download as ZIP" to download entire folders

## 🛠️ Development

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (latest stable)
- [Tauri Prerequisites](https://tauri.app/start/prerequisites/)

### Setup

```bash
# Clone the repository
git clone https://github.com/YOUR_USERNAME/cloudflare-backup.git
cd cloudflare-backup

# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Project Structure

```
├── src/                    # Svelte frontend
│   ├── components/         # UI components
│   ├── lib/               # Stores, types, utilities
│   └── routes/            # Page components
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── db/           # SQLite database
│   │   ├── r2/           # R2 operations
│   │   ├── sync/         # Folder sync logic
│   │   └── upload/       # Upload manager
│   └── migrations/        # Database migrations
└── .github/workflows/     # CI/CD
```

## 🔄 GitHub Actions (For Developers)

The repository includes GitHub Actions workflows to automatically build releases for all platforms.

### Creating a Release

1. Tag your release:
   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

2. GitHub Actions will automatically:
   - Build for macOS, Windows, and Linux
   - Create a draft release with all installers
   - You can then edit and publish the release

### Manual Build Trigger

1. Go to **Actions** → **Build and Release**
2. Click **Run workflow**
3. Optionally specify a version number
4. Click **Run workflow**

## 🛠️ Tech Stack

- **Frontend**: Svelte 5, TypeScript, TailwindCSS 4
- **Backend**: Rust, Tauri 2
- **Storage**: Cloudflare R2 (S3-compatible)
- **Database**: SQLite (via sqlx)
- **Encryption**: AES-256-GCM (via aes-gcm crate)

## 📝 License

MIT

## 🤝 Contributing

Contributions are welcome! Please open an issue or pull request.

---

Made with ❤️ using [Tauri](https://tauri.app/)
