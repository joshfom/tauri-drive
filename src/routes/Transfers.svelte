<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { UploadProgress } from '../lib/types';
  import { formatBytes, formatDuration } from '../lib/utils/formatters';

  let transfers: UploadProgress[] = [];
  let loading = false;

  async function loadActiveUploads() {
    loading = true;
    try {
      transfers = await invoke<UploadProgress[]>('get_active_uploads');
    } catch (e) {
      console.error('Failed to load uploads:', e);
    } finally {
      loading = false;
    }
  }

  async function handleCancel(uploadId: string) {
    try {
      await invoke('cancel_upload', { uploadId });
      await loadActiveUploads();
    } catch (e) {
      console.error('Failed to cancel upload:', e);
    }
  }

  function getStatusColor(status: string): string {
    switch (status) {
      case 'uploading': return 'text-blue-600 dark:text-blue-400';
      case 'completed': return 'text-green-600 dark:text-green-400';
      case 'failed': return 'text-red-600 dark:text-red-400';
      case 'paused': return 'text-yellow-600 dark:text-yellow-400';
      case 'cancelled': return 'text-gray-600 dark:text-gray-400';
      default: return 'text-gray-600 dark:text-gray-400';
    }
  }

  onMount(() => {
    loadActiveUploads();
    // Refresh every 2 seconds
    const interval = setInterval(loadActiveUploads, 2000);
    return () => clearInterval(interval);
  });
</script>

<div class="flex flex-col h-full">
  <!-- Header -->
  <div class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
    <h2 class="text-2xl font-semibold text-gray-900 dark:text-white">Transfers</h2>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-auto p-6">
    {#if loading && transfers.length === 0}
      <div class="flex items-center justify-center h-64">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600"></div>
      </div>
    {:else if transfers.length === 0}
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
        <h3 class="mt-2 text-sm font-medium text-gray-900 dark:text-white">No active transfers</h3>
        <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">
          Upload or download files to see them here.
        </p>
      </div>
    {:else}
      <div class="space-y-4">
        {#each transfers as transfer}
          <div class="bg-white dark:bg-gray-800 shadow rounded-lg p-4">
            <div class="flex items-center justify-between mb-2">
              <div class="flex-1">
                <p class="font-medium text-gray-900 dark:text-white">{transfer.fileName}</p>
                <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
                  {formatBytes(transfer.uploadedSize)} / {formatBytes(transfer.totalSize)}
                </p>
              </div>
              <div class="flex items-center space-x-3">
                <span class="text-sm {getStatusColor(transfer.status.toLowerCase())} font-medium capitalize">
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
                class="bg-blue-600 h-2 rounded-full transition-all duration-300"
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
  </div>
</div>
