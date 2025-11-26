import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import {
  uploadQueue,
  isConnected,
  currentBucket,
  addToQueue,
  updateUploadProgress,
  removeFromQueue,
  clearCompletedUploads,
} from './uploads';
import type { UploadProgress } from '../types';

describe('Upload Store', () => {
  beforeEach(() => {
    // Reset stores before each test
    uploadQueue.set([]);
    isConnected.set(false);
    currentBucket.set('');
  });

  describe('uploadQueue store', () => {
    it('should start with empty queue', () => {
      expect(get(uploadQueue)).toEqual([]);
    });
  });

  describe('isConnected store', () => {
    it('should start as false', () => {
      expect(get(isConnected)).toBe(false);
    });

    it('should update connection status', () => {
      isConnected.set(true);
      expect(get(isConnected)).toBe(true);
    });
  });

  describe('currentBucket store', () => {
    it('should start with empty string', () => {
      expect(get(currentBucket)).toBe('');
    });

    it('should update bucket name', () => {
      currentBucket.set('my-bucket');
      expect(get(currentBucket)).toBe('my-bucket');
    });
  });

  describe('addToQueue', () => {
    it('should add upload to queue', () => {
      const upload: UploadProgress = {
        id: '1',
        fileName: 'test.txt',
        filePath: '/path/to/test.txt',
        remotePath: 'uploads/test.txt',
        totalSize: 1024,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      addToQueue(upload);
      
      const queue = get(uploadQueue);
      expect(queue).toHaveLength(1);
      expect(queue[0]).toEqual(upload);
    });

    it('should add multiple uploads to queue', () => {
      const upload1: UploadProgress = {
        id: '1',
        fileName: 'file1.txt',
        filePath: '/path/file1.txt',
        remotePath: 'file1.txt',
        totalSize: 100,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      const upload2: UploadProgress = {
        id: '2',
        fileName: 'file2.txt',
        filePath: '/path/file2.txt',
        remotePath: 'file2.txt',
        totalSize: 200,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      addToQueue(upload1);
      addToQueue(upload2);

      const queue = get(uploadQueue);
      expect(queue).toHaveLength(2);
    });
  });

  describe('updateUploadProgress', () => {
    it('should update existing upload progress', () => {
      const upload: UploadProgress = {
        id: '1',
        fileName: 'test.txt',
        filePath: '/path/test.txt',
        remotePath: 'test.txt',
        totalSize: 1000,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      addToQueue(upload);
      updateUploadProgress('1', { 
        uploadedSize: 500, 
        progress: 50, 
        speed: 100,
        status: 'uploading' 
      });

      const queue = get(uploadQueue);
      expect(queue[0].uploadedSize).toBe(500);
      expect(queue[0].progress).toBe(50);
      expect(queue[0].speed).toBe(100);
      expect(queue[0].status).toBe('uploading');
    });

    it('should not affect other uploads', () => {
      const upload1: UploadProgress = {
        id: '1',
        fileName: 'file1.txt',
        filePath: '/path/file1.txt',
        remotePath: 'file1.txt',
        totalSize: 100,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      const upload2: UploadProgress = {
        id: '2',
        fileName: 'file2.txt',
        filePath: '/path/file2.txt',
        remotePath: 'file2.txt',
        totalSize: 200,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      addToQueue(upload1);
      addToQueue(upload2);
      updateUploadProgress('1', { status: 'completed', progress: 100 });

      const queue = get(uploadQueue);
      expect(queue[0].status).toBe('completed');
      expect(queue[1].status).toBe('pending');
    });

    it('should handle non-existent upload id', () => {
      const upload: UploadProgress = {
        id: '1',
        fileName: 'test.txt',
        filePath: '/path/test.txt',
        remotePath: 'test.txt',
        totalSize: 100,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      addToQueue(upload);
      updateUploadProgress('non-existent', { status: 'completed' });

      const queue = get(uploadQueue);
      expect(queue).toHaveLength(1);
      expect(queue[0].status).toBe('pending');
    });
  });

  describe('removeFromQueue', () => {
    it('should remove upload from queue', () => {
      const upload: UploadProgress = {
        id: '1',
        fileName: 'test.txt',
        filePath: '/path/test.txt',
        remotePath: 'test.txt',
        totalSize: 100,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      addToQueue(upload);
      expect(get(uploadQueue)).toHaveLength(1);

      removeFromQueue('1');
      expect(get(uploadQueue)).toHaveLength(0);
    });

    it('should only remove specified upload', () => {
      const upload1: UploadProgress = {
        id: '1',
        fileName: 'file1.txt',
        filePath: '/path/file1.txt',
        remotePath: 'file1.txt',
        totalSize: 100,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      const upload2: UploadProgress = {
        id: '2',
        fileName: 'file2.txt',
        filePath: '/path/file2.txt',
        remotePath: 'file2.txt',
        totalSize: 200,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      addToQueue(upload1);
      addToQueue(upload2);
      removeFromQueue('1');

      const queue = get(uploadQueue);
      expect(queue).toHaveLength(1);
      expect(queue[0].id).toBe('2');
    });
  });

  describe('clearCompletedUploads', () => {
    it('should remove all completed uploads', () => {
      const pending: UploadProgress = {
        id: '1',
        fileName: 'pending.txt',
        filePath: '/path/pending.txt',
        remotePath: 'pending.txt',
        totalSize: 100,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      const completed: UploadProgress = {
        id: '2',
        fileName: 'completed.txt',
        filePath: '/path/completed.txt',
        remotePath: 'completed.txt',
        totalSize: 100,
        uploadedSize: 100,
        progress: 100,
        speed: 0,
        eta: 0,
        status: 'completed',
      };

      const uploading: UploadProgress = {
        id: '3',
        fileName: 'uploading.txt',
        filePath: '/path/uploading.txt',
        remotePath: 'uploading.txt',
        totalSize: 100,
        uploadedSize: 50,
        progress: 50,
        speed: 100,
        eta: 5,
        status: 'uploading',
      };

      addToQueue(pending);
      addToQueue(completed);
      addToQueue(uploading);

      clearCompletedUploads();

      const queue = get(uploadQueue);
      expect(queue).toHaveLength(2);
      expect(queue.find(u => u.id === '2')).toBeUndefined();
    });

    it('should handle empty queue', () => {
      clearCompletedUploads();
      expect(get(uploadQueue)).toHaveLength(0);
    });

    it('should handle queue with no completed uploads', () => {
      const upload: UploadProgress = {
        id: '1',
        fileName: 'pending.txt',
        filePath: '/path/pending.txt',
        remotePath: 'pending.txt',
        totalSize: 100,
        uploadedSize: 0,
        progress: 0,
        speed: 0,
        eta: 0,
        status: 'pending',
      };

      addToQueue(upload);
      clearCompletedUploads();

      expect(get(uploadQueue)).toHaveLength(1);
    });
  });
});
