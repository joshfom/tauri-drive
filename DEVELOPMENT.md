# Development Notes

## Current Status

### ✅ Completed (Phase 1 Foundation)

1. **Project Setup**
   - Tauri 2.x with Svelte 5 and TypeScript
   - TailwindCSS for styling
   - svelte-spa-router for navigation
   - Complete folder structure created

2. **Backend (Rust)**
   - R2 client module with S3-compatible API
   - Basic CRUD operations (list, get, put, delete, copy)
   - Multipart upload engine for large files
   - SQLite database integration
   - Database schema for uploads, chunks, sync, metadata
   - Tauri commands for R2 operations

3. **Frontend (Svelte)**
   - Main app layout with sidebar navigation
   - Browser page for file listing
   - Settings page for R2 credentials
   - Transfers page (placeholder)
   - TypeScript types for all data structures

4. **Dependencies**
   - Rust: aws-sdk-s3, tokio, sqlx, notify, serde, anyhow
   - Frontend: Svelte 5, TailwindCSS, svelte-spa-router

### 🚧 Next Steps (Phase 2)

1. **Upload Queue System**
   - State management with Svelte stores
   - Progress tracking and events
   - Pause/Resume/Cancel functionality
   - Real-time progress updates

2. **Enhanced File Operations**
   - Drag & drop upload
   - Multiple file selection
   - File download with progress
   - Delete confirmation dialogs
   - Rename/move operations

3. **Multipart Upload UI**
   - Show chunk progress
   - Resume capability UI
   - Speed and ETA calculations
   - Error handling and retry

4. **Testing**
   - Test with actual R2 bucket
   - Large file uploads (>1GB)
   - Error handling
   - Resume functionality

## How to Test

1. **Start the app**
   ```bash
   npm run tauri dev
   ```

2. **Configure R2**
   - Go to Settings
   - Enter your Cloudflare R2 credentials
   - Click "Connect to R2"

3. **Browse Files**
   - Navigate to Browser
   - Click "Refresh" to load files
   - (Upload functionality needs file picker implementation)

## Known Issues / TODO

- [ ] Add file picker dialog for uploads
- [ ] Implement download functionality
- [ ] Add delete confirmation
- [ ] Implement progress tracking
- [ ] Add error notifications
- [ ] Implement virtual scrolling for large file lists
- [ ] Add file preview modal
- [ ] Implement dark mode toggle
- [ ] Add system tray icon

## Architecture Notes

### Multipart Upload Flow
1. User selects file > 100MB
2. Frontend calls `upload_file` command
3. Backend creates multipart upload session
4. File is split into 10MB chunks
5. Chunks uploaded in parallel (up to 6 concurrent)
6. Progress updates sent to frontend via events
7. Upload completion triggers UI update
8. State saved to SQLite for resume capability

### Database Usage
- SQLite stores upload state for resume
- File metadata cached to reduce API calls
- Sync configuration persisted
- Credentials stored (will be encrypted)

### Future: Sync Engine
- File system watcher monitors local folder
- Change detection compares with R2 state
- Delta sync uploads only changed files
- Conflict resolution UI for bidirectional sync
