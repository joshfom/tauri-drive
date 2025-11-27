<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { downloadQueue, addToDownloadQueue, updateDownloadProgress, removeFromDownloadQueue } from '$lib/stores/uploads';
  import { formatBytes, formatDuration } from '$lib/utils/formatters';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { isPermissionGranted, sendNotification } from '@tauri-apps/plugin-notification';
  import type { DownloadProgress } from '$lib/types';

  let isExpanded = true;
  let unlisten: UnlistenFn | null = null;
  let notificationsEnabled = false;
  let completedIds = new Set<string>();

  // Check notification permission
  async function initNotifications() {
    try {
      notificationsEnabled = await isPermissionGranted();
    } catch (e) {
      console.error('Failed to check notification permission:', e);
      notificationsEnabled = false;
    }
  }

  // Send notification for completed download
  async function notifyDownloadComplete(fileName: string) {
    if (!notificationsEnabled) return;
    try {
      sendNotification({
        title: 'Download Complete',
        body: `${fileName} has been downloaded successfully.`,
      });
    } catch (e) {
      console.error('Failed to send notification:', e);
    }
  }

  onMount(async () => {
    await initNotifications();

    // Listen for download progress events from Rust
    unlisten = await listen<DownloadProgress>('download-progress', (event) => {
      const progress = event.payload;
      console.log('Download progress received:', progress);
      
      // Check if download already exists in queue
      const exists = $downloadQueue.some(d => d.id === progress.id);
      
      if (!exists) {
        console.log('Adding new download to queue:', progress.id);
        addToDownloadQueue(progress);
      } else {
        console.log('Updating existing download:', progress.id, progress.progress.toFixed(1) + '%');
        updateDownloadProgress(progress.id, progress);
      }

      // Check for completed download and send notification
      if (progress.status === 'completed' && !completedIds.has(progress.id)) {
        completedIds.add(progress.id);
        notifyDownloadComplete(progress.fileName);
      }
    });
  });

  onDestroy(() => {
    if (unlisten) {
      unlisten();
    }
  });

  function handleDismiss(downloadId: string) {
    removeFromDownloadQueue(downloadId);
  }

  function getStatusColor(status: string): string {
    switch (status.toLowerCase()) {
      case 'downloading': return 'bg-purple-600';
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
      case 'downloading': return 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400';
      case 'completed': return 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400';
      case 'failed': return 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400';
      case 'cancelled': return 'bg-gray-100 text-gray-700 dark:bg-gray-700/30 dark:text-gray-400';
      case 'paused': return 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400';
      case 'pending': return 'bg-gray-100 text-gray-600 dark:bg-gray-700/30 dark:text-gray-400';
      default: return 'bg-gray-100 text-gray-700 dark:bg-gray-700/30 dark:text-gray-400';
    }
  }

  $: activeDownloads = $downloadQueue.filter(d => d.status !== 'completed');
  $: hasActiveDownloads = activeDownloads.length > 0;
  $: downloadingCount = activeDownloads.filter(d => d.status === 'downloading').length;
  $: failedCount = activeDownloads.filter(d => d.status === 'failed' || d.status === 'cancelled').length;
</script>

{#if hasActiveDownloads}
  <div class="fixed bottom-4 left-4 w-96 bg-white/95 dark:bg-gray-800/95 backdrop-blur-md border border-gray-200/50 dark:border-gray-700/50 shadow-2xl rounded-2xl z-50 overflow-hidden">
    <!-- Header -->
    <div class="px-5 py-4 bg-gradient-to-r from-purple-50 to-purple-100 dark:from-purple-900/30 dark:to-purple-800/30 flex items-center justify-between border-b border-gray-200/50 dark:border-gray-700/50">
      <div class="flex items-center space-x-3">
        <div class="w-8 h-8 bg-gradient-to-br from-purple-500 to-purple-600 rounded-lg flex items-center justify-center shadow-lg">
          <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
          </svg>
        </div>
        <div>
          <span class="font-semibold text-gray-900 dark:text-white text-sm">
            Downloads
          </span>
          <p class="text-xs text-gray-600 dark:text-gray-400">
            {#if downloadingCount > 0}
              {downloadingCount} downloading
            {/if}
            {#if failedCount > 0}
              {downloadingCount > 0 ? ', ' : ''}{failedCount} failed
            {/if}
            {#if downloadingCount === 0 && failedCount === 0}
              {activeDownloads.length} pending
            {/if}
          </p>
        </div>
      </div>
      <button
        on:click={() => isExpanded = !isExpanded}
        title="Toggle download queue"
        class="p-1.5 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-lg hover:bg-white/50 dark:hover:bg-gray-700/50 transition-colors duration-150"
      >
        <svg class="w-5 h-5 transform transition-transform duration-200 {isExpanded ? '' : 'rotate-180'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
        </svg>
      </button>
    </div>

    {#if isExpanded}
      <!-- Download list -->
      <div class="max-h-80 overflow-y-auto p-4 space-y-3">
        {#each activeDownloads as download}
          <div class="bg-gradient-to-r from-gray-50 to-gray-100 dark:from-gray-700/50 dark:to-gray-800/50 rounded-xl p-4 border border-gray-200/50 dark:border-gray-600/50 shadow-sm">
            <div class="flex items-start justify-between mb-3">
              <div class="flex items-start space-x-3 flex-1 min-w-0">
                <div class="w-10 h-10 bg-gradient-to-br from-purple-100 to-purple-200 dark:from-purple-900/40 dark:to-purple-800/40 rounded-lg flex items-center justify-center shrink-0">
                  {#if download.status === 'failed' || download.status === 'cancelled'}
                    <svg class="w-5 h-5 text-red-600 dark:text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                    </svg>
                  {:else}
                    <svg class="w-5 h-5 text-purple-600 dark:text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                    </svg>
                  {/if}
                </div>
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-semibold text-gray-900 dark:text-white truncate">
                    {download.fileName}
                  </p>
                  <p class="text-xs text-gray-600 dark:text-gray-400 mt-1">
                    {formatBytes(download.downloadedSize)} / {formatBytes(download.totalSize)}
                  </p>
                  {#if download.status === 'downloading' && (download.speed > 0 || download.eta > 0)}
                    <div class="flex items-center space-x-2 mt-1">
                      {#if download.speed > 0}
                        <span class="text-xs text-purple-600 dark:text-purple-400 font-medium">
                          {formatBytes(download.speed)}/s
                        </span>
                      {/if}
                      {#if download.eta > 0}
                        <span class="text-xs text-gray-500 dark:text-gray-400">
                          · {formatDuration(download.eta)} left
                        </span>
                      {/if}
                    </div>
                  {/if}
                </div>
              </div>
              
              <!-- Dismiss button for failed/cancelled -->
              {#if download.status === 'failed' || download.status === 'cancelled'}
                <button
                  on:click={() => handleDismiss(download.id)}
                  class="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50 rounded-lg transition-colors duration-150 ml-2"
                  title="Dismiss"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                  </svg>
                </button>
              {/if}
            </div>

            <!-- Progress bar -->
            <div class="relative w-full bg-gray-200 dark:bg-gray-600 rounded-full h-2 overflow-hidden">
              <div
                class="{getStatusColor(download.status)} h-2 rounded-full transition-all duration-300 shadow-sm"
                style="width: {download.progress}%"
              ></div>
            </div>

            <div class="flex items-center justify-between mt-2">
              <span class="text-xs font-semibold text-gray-700 dark:text-gray-300">
                {download.progress.toFixed(1)}%
              </span>
              <span class="text-xs px-2 py-0.5 rounded-full font-medium {getStatusBadgeClass(download.status)}">
                {download.status}
              </span>
            </div>

            {#if download.errorMessage}
              <div class="mt-3 p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800/50 rounded-lg">
                <p class="text-xs text-red-700 dark:text-red-400">{download.errorMessage}</p>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}
