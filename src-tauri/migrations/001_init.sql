-- Buckets/accounts configuration
CREATE TABLE IF NOT EXISTS buckets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    account_id TEXT NOT NULL,
    region TEXT,
    endpoint TEXT NOT NULL,
    access_key_id TEXT NOT NULL,
    secret_access_key TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Upload tracking
CREATE TABLE IF NOT EXISTS uploads (
    id TEXT PRIMARY KEY,
    bucket_id INTEGER,
    file_path TEXT NOT NULL,
    remote_path TEXT NOT NULL,
    total_size INTEGER NOT NULL,
    uploaded_size INTEGER DEFAULT 0,
    chunk_size INTEGER NOT NULL,
    upload_id TEXT,
    status TEXT CHECK(status IN ('pending', 'uploading', 'paused', 'completed', 'failed', 'cancelled')) DEFAULT 'pending',
    error_message TEXT,
    started_at DATETIME,
    completed_at DATETIME,
    FOREIGN KEY (bucket_id) REFERENCES buckets(id) ON DELETE SET NULL
);

-- Upload chunks tracking for resume capability
CREATE TABLE IF NOT EXISTS upload_chunks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    upload_id TEXT NOT NULL,
    part_number INTEGER NOT NULL,
    size INTEGER NOT NULL,
    etag TEXT,
    status TEXT CHECK(status IN ('pending', 'uploading', 'completed', 'failed')) DEFAULT 'pending',
    uploaded_at DATETIME,
    FOREIGN KEY (upload_id) REFERENCES uploads(id) ON DELETE CASCADE,
    UNIQUE(upload_id, part_number)
);

-- Sync folder configuration
CREATE TABLE IF NOT EXISTS sync_folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bucket_id INTEGER NOT NULL,
    local_path TEXT NOT NULL,
    remote_path TEXT NOT NULL,
    sync_mode TEXT CHECK(sync_mode IN ('upload', 'download', 'bidirectional')) DEFAULT 'bidirectional',
    enabled BOOLEAN DEFAULT 1,
    last_sync DATETIME,
    FOREIGN KEY (bucket_id) REFERENCES buckets(id) ON DELETE CASCADE
);

-- File metadata cache
CREATE TABLE IF NOT EXISTS file_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bucket_id INTEGER NOT NULL,
    remote_path TEXT NOT NULL,
    size INTEGER NOT NULL,
    etag TEXT,
    last_modified DATETIME,
    content_type TEXT,
    cached_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (bucket_id) REFERENCES buckets(id) ON DELETE CASCADE,
    UNIQUE(bucket_id, remote_path)
);

-- Application settings
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_uploads_status ON uploads(status);
CREATE INDEX IF NOT EXISTS idx_upload_chunks_upload_id ON upload_chunks(upload_id);
CREATE INDEX IF NOT EXISTS idx_file_metadata_bucket ON file_metadata(bucket_id, remote_path);
CREATE INDEX IF NOT EXISTS idx_sync_folders_enabled ON sync_folders(enabled);
