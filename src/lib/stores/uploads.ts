import { writable } from 'svelte/store';
import type { UploadProgress, DownloadProgress, TransferQueueItem } from '../types';

export const uploadQueue = writable<UploadProgress[]>([]);
export const downloadQueue = writable<DownloadProgress[]>([]);
export const isConnected = writable<boolean>(false);
export const currentBucket = writable<string>('');
export const currentBrowserPath = writable<string>(''); // Cache the current browser folder path

// Upload functions
export function addToQueue(upload: UploadProgress) {
  uploadQueue.update(queue => [...queue, upload]);
}

export function updateUploadProgress(uploadId: string, progress: Partial<UploadProgress>) {
  uploadQueue.update(queue => 
    queue.map(item => {
      if (item.id !== uploadId) return item;
      
      // Only update progress if it's higher (prevent jumping back)
      const newProgress = progress.progress !== undefined 
        ? Math.max(item.progress, progress.progress) 
        : item.progress;
      
      const newUploadedSize = progress.uploadedSize !== undefined
        ? Math.max(item.uploadedSize, progress.uploadedSize)
        : item.uploadedSize;
      
      return { 
        ...item, 
        ...progress,
        progress: newProgress,
        uploadedSize: newUploadedSize
      };
    })
  );
}

export function removeFromQueue(uploadId: string) {
  uploadQueue.update(queue => queue.filter(item => item.id !== uploadId));
}

export function clearCompletedUploads() {
  uploadQueue.update(queue => 
    queue.filter(item => item.status !== 'completed')
  );
}

// Download functions
export function addToDownloadQueue(download: DownloadProgress) {
  downloadQueue.update(queue => {
    // Check if already exists
    const exists = queue.some(d => d.id === download.id);
    if (exists) return queue;
    return [...queue, download];
  });
}

export function updateDownloadProgress(downloadId: string, progress: Partial<DownloadProgress>) {
  downloadQueue.update(queue => 
    queue.map(item => {
      if (item.id !== downloadId) return item;
      
      // Only update progress if it's higher (prevent jumping back)
      const newProgress = progress.progress !== undefined 
        ? Math.max(item.progress, progress.progress) 
        : item.progress;
      
      const newDownloadedSize = progress.downloadedSize !== undefined
        ? Math.max(item.downloadedSize, progress.downloadedSize)
        : item.downloadedSize;
      
      return { 
        ...item, 
        ...progress,
        progress: newProgress,
        downloadedSize: newDownloadedSize
      };
    })
  );
}

export function removeFromDownloadQueue(downloadId: string) {
  downloadQueue.update(queue => queue.filter(item => item.id !== downloadId));
}

export function clearCompletedDownloads() {
  downloadQueue.update(queue => 
    queue.filter(item => item.status !== 'completed')
  );
}
