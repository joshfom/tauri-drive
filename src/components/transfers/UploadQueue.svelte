<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { uploadQueue, updateUploadProgress, addToQueue, removeFromQueue } from '$lib/stores/uploads';
  import { formatBytes, formatDuration } from '$lib/utils/formatters';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import type { UploadProgress } from '$lib/types';

  let isExpanded = true;
  let unlisten: UnlistenFn | null = null;

  onMount(async () => {
    // Listen for upload progress events from Rust
    unlisten = await listen<UploadProgress>('upload-progress', (event) => {
      const progress = event.payload;
      console.log('Upload progress received:', progress);
      
      // Check if upload already exists in queue
      const exists = $uploadQueue.some(u => u.id === progress.id);
      
      if (!exists) {
        // Add new upload to queue
        console.log('Adding new upload to queue:', progress.id);
        addToQueue(progress);
      } else {
        // Update existing upload
        console.log('Updating existing upload:', progress.id, progress.progress.toFixed(1) + '%');
        updateUploadProgress(progress.id, progress);
      }
    });
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });

  async function handleCancel(uploadId: string) {
    try {
      await invoke('cancel_upload', { uploadId });
      // Update the local state to show cancelled
      updateUploadProgress(uploadId, { status: 'cancelled' });
    } catch (e) {
      console.error('Failed to cancel upload:', e);
    }
  }

  async function handlePause(uploadId: string) {
    try {
      await invoke('pause_upload', { uploadId });
      updateUploadProgress(uploadId, { status: 'paused' });
    } catch (e) {
      console.error('Failed to pause upload:', e);
    }
  }

  async function handleResume(uploadId: string) {
    try {
      await invoke('resume_upload', { uploadId });
      updateUploadProgress(uploadId, { status: 'uploading' });
    } catch (e) {
      console.error('Failed to resume upload:', e);
    }
  }

  async function handleRetry(uploadId: string) {
    try {
      // Remove the failed upload from the queue first
      removeFromQueue(uploadId);
      // Then retry - this will create a new upload with a new ID
      await invoke('retry_upload', { uploadId });
    } catch (e) {
      console.error('Failed to retry upload:', e);
    }
  }

  function handleDismiss(uploadId: string) {
    removeFromQueue(uploadId);
  }

  function getStatusColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'uploading': return 'bg-blue-600';
      case 'completed': return 'bg-green-600';
      case 'failed': return 'bg-red-600';
      case 'cancelled': return 'bg-gray-500';
      case 'paused': return 'bg-yellow-600';
      case 'pending': return 'bg-gray-400';
      default: return 'bg-gray-600';
    }
  }

  function getStatusBadgeClass(status: string): string {
    switch (status.toLowerCase()) {
      case 'uploading': return 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400';
      case 'completed': return 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400';
      case 'failed': return 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400';
      case 'cancelled': return 'bg-gray-100 text-gray-700 dark:bg-gray-700/30 dark:text-gray-400';
      case 'paused': return 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400';
      case 'pending': return 'bg-gray-100 text-gray-600 dark:bg-gray-700/30 dark:text-gray-400';
      default: return 'bg-gray-100 text-gray-700 dark:bg-gray-700/30 dark:text-gray-400';
    }
  }

  $: activeUploads = $uploadQueue.filter(u => u.status !== 'completed');
  $: hasActiveUploads = activeUploads.length > 0;
  $: uploadingCount = activeUploads.filter(u => u.status === 'uploading').length;
  $: pausedCount = activeUploads.filter(u => u.status === 'paused').length;
  $: failedCount = activeUploads.filter(u => u.status === 'failed' || u.status === 'cancelled').length;
</script>

{#if hasActiveUploads}
  <div class="fixed bottom-4 right-4 w-96 bg-white/95 dark:bg-gray-800/95 backdrop-blur-md border border-gray-200/50 dark:border-gray-700/50 shadow-2xl rounded-2xl z-50 overflow-hidden">
    <!-- Header -->
    <div class="px-5 py-4 bg-gradient-to-r from-blue-50 to-blue-100 dark:from-blue-900/30 dark:to-blue-800/30 flex items-center justify-between border-b border-gray-200/50 dark:border-gray-700/50">
      <div class="flex items-center space-x-3">
        <div class="w-8 h-8 bg-gradient-to-br from-blue-500 to-blue-600 rounded-lg flex items-center justify-center shadow-lg">
          <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
          </svg>
        </div>
        <div>
          <span class="font-semibold text-gray-900 dark:text-white text-sm">
            Transfers
          </span>
          <p class="text-xs text-gray-600 dark:text-gray-400">
            {#if uploadingCount > 0}
              {uploadingCount} uploading
            {/if}
            {#if pausedCount > 0}
              {uploadingCount > 0 ? ', ' : ''}{pausedCount} paused
            {/if}
            {#if failedCount > 0}
              {(uploadingCount > 0 || pausedCount > 0) ? ', ' : ''}{failedCount} failed
            {/if}
            {#if uploadingCount === 0 && pausedCount === 0 && failedCount === 0}
              {activeUploads.length} pending
            {/if}
          </p>
        </div>
      </div>
      <button
        on:click={() => isExpanded = !isExpanded}
        title="Toggle upload queue"
        class="p-1.5 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-lg hover:bg-white/50 dark:hover:bg-gray-700/50 transition-colors duration-150"
      >
        <svg class="w-5 h-5 transform transition-transform duration-200 {isExpanded ? '' : 'rotate-180'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
        </svg>
      </button>
    </div>

    {#if isExpanded}
      <!-- Upload list -->
      <div class="max-h-80 overflow-y-auto p-4 space-y-3">
        {#each activeUploads as upload}
          <div class="bg-gradient-to-r from-gray-50 to-gray-100 dark:from-gray-700/50 dark:to-gray-800/50 rounded-xl p-4 border border-gray-200/50 dark:border-gray-600/50 shadow-sm">
            <div class="flex items-start justify-between mb-3">
              <div class="flex items-start space-x-3 flex-1 min-w-0">
                <div class="w-10 h-10 bg-gradient-to-br from-blue-100 to-blue-200 dark:from-blue-900/40 dark:to-blue-800/40 rounded-lg flex items-center justify-center flex-shrink-0">
                  {#if upload.status === 'paused'}
                    <svg class="w-5 h-5 text-yellow-600 dark:text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                  {:else if upload.status === 'failed' || upload.status === 'cancelled'}
                    <svg class="w-5 h-5 text-red-600 dark:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                    </svg>
                  {:else}
                    <svg class="w-5 h-5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                    </svg>
                  {/if}
                </div>
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-semibold text-gray-900 dark:text-white truncate">
                    {upload.fileName}
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
                    {formatBytes(upload.uploadedSize)} / {formatBytes(upload.totalSize)}
                  </p>
                  {#if upload.status === 'uploading' && (upload.speed > 0 || upload.eta > 0)}
                    <div class="flex items-center space-x-2 mt-1">
                      {#if upload.speed > 0}
                        <span class="text-xs text-blue-600 dark:text-blue-400 font-medium">
                          {formatBytes(upload.speed)}/s
                        </span>
                      {/if}
                      {#if upload.eta > 0}
                        <span class="text-xs text-gray-500 dark:text-gray-400">
                          · {formatDuration(upload.eta)} left
                        </span>
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>
              
              <!-- Action buttons -->
              <div class="flex items-center space-x-1 ml-2">
                {#if upload.status === 'uploading'}
                  <!-- Pause button -->
                  <button
                    on:click={() => handlePause(upload.id)}
                    class="p-1.5 text-gray-400 hover:text-yellow-600 dark:hover:text-yellow-400 hover:bg-yellow-50 dark:hover:bg-yellow-900/20 rounded-lg transition-colors duration-150"
                    title="Pause upload"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6"/>
                    </svg>
                  </button>
                {:else if upload.status === 'paused'}
                  <!-- Resume button -->
                  <button
                    on:click={() => handleResume(upload.id)}
                    class="p-1.5 text-gray-400 hover:text-green-600 dark:hover:text-green-400 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg transition-colors duration-150"
                    title="Resume upload"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/>
                    </svg>
                  </button>
                {:else if upload.status === 'failed' || upload.status === 'cancelled'}
                  <!-- Retry button -->
                  <button
                    on:click={() => handleRetry(upload.id)}
                    class="p-1.5 text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 hover:bg-blue-50 dark:hover:bg-blue-900/20 rounded-lg transition-colors duration-150"
                    title="Retry upload"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                  </button>
                  <!-- Dismiss button for failed/cancelled -->
                  <button
                    on:click={() => handleDismiss(upload.id)}
                    class="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50 rounded-lg transition-colors duration-150"
                    title="Dismiss"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                  </button>
                {/if}
                
                {#if upload.status === 'uploading' || upload.status === 'paused' || upload.status === 'pending'}
                  <!-- Cancel button -->
                  <button
                    on:click={() => handleCancel(upload.id)}
                    class="p-1.5 text-gray-400 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors duration-150"
                    title="Cancel upload"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                  </button>
                {/if}
              </div>
            </div>

            <!-- Progress bar -->
            <div class="relative w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2 overflow-hidden">
              <div
                class="{getStatusColor(upload.status)} h-2 rounded-full transition-all duration-300 shadow-sm"
                style="width: {upload.progress}%"
              ></div>
            </div>

            <div class="flex items-center justify-between mt-2">
              <span class="text-xs font-semibold text-gray-700 dark:text-gray-300">
                {upload.progress.toFixed(1)}%
              </span>
              <span class="text-xs px-2 py-0.5 rounded-full font-medium {getStatusBadgeClass(upload.status)}">
                {upload.status}
              </span>
            </div>

            {#if upload.errorMessage}
              <div class="mt-3 p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800/50 rounded-lg">
                <p class="text-xs text-red-700 dark:text-red-400">{upload.errorMessage}</p>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}
