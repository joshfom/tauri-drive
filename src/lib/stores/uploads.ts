import { writable } from 'svelte/store';
import type { UploadProgress, TransferQueueItem } from '../types';

export const uploadQueue = writable<UploadProgress[]>([]);
export const isConnected = writable<boolean>(false);
export const currentBucket = writable<string>('');
export const currentBrowserPath = writable<string>(''); // Cache the current browser folder path

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
