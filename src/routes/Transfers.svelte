<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import type { UploadProgress, DownloadProgress } from '../lib/types';
  import { formatBytes, formatDuration } from '../lib/utils/formatters';
  import { uploadQueue } from '../lib/stores/uploads';

  // Use the shared upload queue store
  $: uploads = $uploadQueue;
  
  let downloads: Map<string, DownloadProgress> = new Map();
  let activeTab: 'uploads' | 'downloads' = 'uploads';
  let unlistenDownload: UnlistenFn | null = null;

  $: downloadsList = Array.from(downloads.values());
  $: activeDownloads = downloadsList.filter(d => d.status === 'downloading');
  $: activeUploads = uploads.filter(u => u.status.toLowerCase() === 'uploading');

  async function handleCancel(uploadId: string) {
    try {
      await invoke('cancel_upload', { uploadId });
    } catch (e) {
      console.error('Failed to cancel upload:', e);
    }
  }

  function getStatusColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'uploading':
      case 'downloading': 
        return 'text-blue-600 dark:text-blue-400';
      case 'completed': return 'text-green-600 dark:text-green-400';
      case 'failed': return 'text-red-600 dark:text-red-400';
      case 'paused': return 'text-yellow-600 dark:text-yellow-400';
      case 'cancelled': return 'text-gray-600 dark:text-gray-400';
      default: return 'text-gray-600 dark:text-gray-400';
    }
  }

  function getProgressBarColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'uploading': return 'bg-blue-600';
      case 'downloading': return 'bg-purple-600';
      case 'completed': return 'bg-green-600';
      case 'failed': return 'bg-red-600';
      default: return 'bg-blue-600';
    }
  }

  onMount(() => {
    // Listen for download progress events
    listen<DownloadProgress>('download-progress', (event) => {
      const progress = event.payload;
      if (progress.status === 'completed') {
        // Keep completed downloads for a bit, then remove
        downloads.set(progress.id, progress);
        downloads = downloads;
        setTimeout(() => {
          downloads.delete(progress.id);
          downloads = downloads;
        }, 5000);
      } else {
        downloads.set(progress.id, progress);
        downloads = downloads;
      }
    }).then((unlisten) => {
      unlistenDownload = unlisten;
    });
  });

  onDestroy(() => {
    if (unlistenDownload) {
      unlistenDownload();
    }
  });
</script>

<div class="flex flex-col h-full">
  <!-- Header -->
  <div class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
    <div class="flex items-center justify-between">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">Transfers</h2>
      <div class="flex items-center space-x-2 text-sm">
        {#if activeUploads.length > 0}
          <span class="px-2 py-1 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 rounded-lg">
            {activeUploads.length} uploading
          </span>
        {/if}
        {#if activeDownloads.length > 0}
          <span class="px-2 py-1 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-400 rounded-lg">
            {activeDownloads.length} downloading
          </span>
        {/if}
      </div>
    </div>
    
    <!-- Tabs -->
    <div class="flex space-x-4 mt-4">
      <button
        on:click={() => activeTab = 'uploads'}
        class="px-4 py-2 text-sm font-medium rounded-lg transition-colors {activeTab === 'uploads' ? 'bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400' : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        Uploads ({uploads.length})
      </button>
      <button
        on:click={() => activeTab = 'downloads'}
        class="px-4 py-2 text-sm font-medium rounded-lg transition-colors {activeTab === 'downloads' ? 'bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-400' : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'}"
      >
        Downloads ({downloadsList.length})
      </button>
    </div>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-auto p-6">
    {#if activeTab === 'uploads'}
      {#if uploads.length === 0}
        <div class="text-center py-12">
          <svg
            class="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"
            />
          </svg>
          <h3 class="mt-2 text-sm font-medium text-gray-900 dark:text-white">No uploads</h3>
          <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Upload files from the browser to see them here.
          </p>
        </div>
      {:else}
        <div class="space-y-4">
          {#each uploads as transfer}
            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-4">
              <div class="flex items-center justify-between mb-2">
                <div class="flex-1">
                  <div class="flex items-center">
                    <svg class="w-4 h-4 text-blue-500 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                    </svg>
                    <p class="font-medium text-gray-900 dark:text-white">{transfer.fileName}</p>
                  </div>
                  <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                    {formatBytes(transfer.uploadedSize)} / {formatBytes(transfer.totalSize)}
                  </p>
                </div>
                <div class="flex items-center space-x-3">
                  <span class="text-sm {getStatusColor(transfer.status)} font-medium capitalize">
                    {transfer.status.toLowerCase()}
                  </span>
                  {#if transfer.status.toLowerCase() === 'uploading' || transfer.status.toLowerCase() === 'pending'}
                    <button
                      on:click={() => handleCancel(transfer.id)}
                      class="px-3 py-1 text-sm bg-red-100 text-red-700 rounded hover:bg-red-200 dark:bg-red-900/20 dark:text-red-400 dark:hover:bg-red-900/30"
                    >
                      Cancel
                    </button>
                  {/if}
                </div>
              </div>
              
              <!-- Progress bar -->
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div
                  class="{getProgressBarColor(transfer.status)} h-2 rounded-full transition-all duration-300"
                  style="width: {transfer.progress}%"
                ></div>
              </div>
              
              <div class="flex items-center justify-between mt-2 text-xs text-gray-500 dark:text-gray-400">
                <span>{transfer.progress.toFixed(1)}%</span>
                {#if transfer.speed > 0}
                  <span>{formatBytes(transfer.speed)}/s</span>
                {/if}
                {#if transfer.eta > 0}
                  <span>ETA: {formatDuration(transfer.eta)}</span>
                {/if}
              </div>

              {#if transfer.errorMessage}
                <div class="mt-2 p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded">
                  <p class="text-sm text-red-800 dark:text-red-200">{transfer.errorMessage}</p>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {:else}
      <!-- Downloads Tab -->
      {#if downloadsList.length === 0}
        <div class="text-center py-12">
          <svg
            class="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"
            />
          </svg>
          <h3 class="mt-2 text-sm font-medium text-gray-900 dark:text-white">No downloads</h3>
          <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">
            Download files from the browser to see them here.
          </p>
        </div>
      {:else}
        <div class="space-y-4">
          {#each downloadsList as download}
            <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-4">
              <div class="flex items-center justify-between mb-2">
                <div class="flex-1">
                  <div class="flex items-center">
                    <svg class="w-4 h-4 text-purple-500 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                    </svg>
                    <p class="font-medium text-gray-900 dark:text-white">{download.fileName}</p>
                  </div>
                  <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                    {formatBytes(download.downloadedSize)} / {formatBytes(download.totalSize)}
                  </p>
                </div>
                <div class="flex items-center space-x-3">
                  <span class="text-sm {getStatusColor(download.status)} font-medium capitalize">
                    {download.status}
                  </span>
                </div>
              </div>
              
              <!-- Progress bar -->
              <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                <div
                  class="{getProgressBarColor(download.status)} h-2 rounded-full transition-all duration-300"
                  style="width: {download.progress}%"
                ></div>
              </div>
              
              <div class="flex items-center justify-between mt-2 text-xs text-gray-500 dark:text-gray-400">
                <span>{download.progress.toFixed(1)}%</span>
                {#if download.speed > 0}
                  <span>{formatBytes(download.speed)}/s</span>
                {/if}
                {#if download.eta > 0}
                  <span>ETA: {formatDuration(download.eta)}</span>
                {/if}
              </div>

              {#if download.errorMessage}
                <div class="mt-2 p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded">
                  <p class="text-sm text-red-800 dark:text-red-200">{download.errorMessage}</p>
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>
