import { writable } from 'svelte/store';
import type { UploadProgress, TransferQueueItem } from '../types';

export const uploadQueue = writable<UploadProgress[]>([]);
export const isConnected = writable<boolean>(false);
export const currentBucket = writable<string>('');

export function addToQueue(upload: UploadProgress) {
  uploadQueue.update(queue => [...queue, upload]);
}

export function updateUploadProgress(uploadId: string, progress: Partial<UploadProgress>) {
  uploadQueue.update(queue => 
    queue.map(item => 
      item.id === uploadId ? { ...item, ...progress } : item
    )
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
