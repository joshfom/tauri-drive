export function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0 || isNaN(bytes)) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

export function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}

export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${Math.round(seconds)}s`;
  if (seconds < 3600) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.round(seconds % 60);
    return `${mins}m ${secs}s`;
  }
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${mins}m`;
}

export function formatDate(date: string | Date): string {
  return new Date(date).toLocaleString();
}

export function calculateETA(totalBytes: number, uploadedBytes: number, bytesPerSecond: number): number {
  if (bytesPerSecond === 0) return 0;
  const remainingBytes = totalBytes - uploadedBytes;
  return Math.round(remainingBytes / bytesPerSecond);
}

/**
 * Extract file name from a path, handling both Windows and Unix paths
 * @param path - Full file path (can be Windows C:\... or Unix /...)
 * @returns Just the file name without the directory path
 */
export function extractFileName(path: string): string {
  if (!path) return 'unknown';
  // Normalize Windows backslashes to forward slashes first
  const normalizedPath = path.replace(/\\/g, '/');
  const parts = normalizedPath.split('/');
  return parts[parts.length - 1] || path;
}

/**
 * Extract folder name from a path, handling both Windows and Unix paths
 * @param path - Full folder path (can be Windows C:\... or Unix /...)
 * @returns Just the folder name without the parent path
 */
export function extractFolderName(path: string): string {
  if (!path) return 'unknown';
  // Normalize Windows backslashes to forward slashes first
  const normalizedPath = path.replace(/\\/g, '/').replace(/\/$/, ''); // Remove trailing slash
  const parts = normalizedPath.split('/');
  return parts[parts.length - 1] || path;
}

export function getFileExtension(filename: string): string {
  const parts = filename.split('.');
  return parts.length > 1 ? parts[parts.length - 1].toLowerCase() : '';
}

export function isImageFile(filename: string): boolean {
  const ext = getFileExtension(filename);
  return ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp'].includes(ext);
}

export function isVideoFile(filename: string): boolean {
  const ext = getFileExtension(filename);
  return ['mp4', 'webm', 'mov', 'avi', 'mkv', 'flv', 'wmv'].includes(ext);
}

export function getFileIcon(filename: string): string {
  if (isImageFile(filename)) return '🖼️';
  if (isVideoFile(filename)) return '🎥';
  
  const ext = getFileExtension(filename);
  switch (ext) {
    case 'pdf': return '📄';
    case 'doc':
    case 'docx': return '📝';
    case 'xls':
    case 'xlsx': return '📊';
    case 'zip':
    case 'rar':
    case 'tar':
    case 'gz': return '📦';
    case 'mp3':
    case 'wav':
    case 'flac': return '🎵';
    default: return '📄';
  }
}
