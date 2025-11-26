// TypeScript types for R2 operations

export interface R2Bucket {
  name: string;
  region?: string;
  creationDate?: string;
}

export interface R2Object {
  key: string;
  size: number;
  lastModified: string;
  etag: string;
  storageClass?: string;
  isDirectory?: boolean;
}

export interface R2Credentials {
  accountId: string;
  accessKeyId: string;
  secretAccessKey: string;
  endpoint: string;
}

export interface UploadProgress {
  id: string;
  fileName: string;
  filePath: string;
  remotePath: string;
  totalSize: number;
  uploadedSize: number;
  progress: number;
  speed: number;
  eta: number;
  status: 'pending' | 'uploading' | 'paused' | 'completed' | 'failed' | 'cancelled';
  errorMessage?: string;
}

export interface TransferQueueItem {
  id: string;
  type: 'upload' | 'download';
  fileName: string;
  localPath: string;
  remotePath: string;
  size: number;
  progress: number;
  status: string;
  speed: number;
  eta: number;
}

export interface SyncFolder {
  id: number;
  localPath: string;
  remotePath: string;
  bucketName: string;
  syncMode: 'upload' | 'download' | 'bidirectional';
  enabled: boolean;
  lastSync?: string;
}

export interface AppSettings {
  theme: 'light' | 'dark' | 'system';
  chunkSize: number;
  parallelUploads: number;
  bandwidthLimit: number;
  conflictResolution: 'ask' | 'local' | 'remote' | 'newest';
  notifications: boolean;
}
