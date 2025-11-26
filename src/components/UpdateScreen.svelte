<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { check, type Update } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';

  export let update: Update;
  
  const dispatch = createEventDispatcher<{
    close: void;
    later: void;
  }>();

  let downloading = false;
  let downloadProgress = 0;
  let downloadedBytes = 0;
  let totalBytes = 0;
  let updateReady = false;
  let error = '';

  // Parse the release notes/body for display
  $: releaseNotes = update?.body || 'No release notes available.';
  $: version = update?.version || 'Unknown';

  async function handleDownloadAndInstall() {
    downloading = true;
    error = '';
    downloadProgress = 0;

    try {
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            totalBytes = event.data.contentLength || 0;
            downloadedBytes = 0;
            break;
          case 'Progress':
            downloadedBytes += event.data.chunkLength || 0;
            if (totalBytes > 0) {
              downloadProgress = Math.round((downloadedBytes / totalBytes) * 100);
            }
            break;
          case 'Finished':
            downloadProgress = 100;
            updateReady = true;
            break;
        }
      });

      updateReady = true;
    } catch (e) {
      error = `Failed to download update: ${e}`;
      downloading = false;
    }
  }

  async function handleRestart() {
    try {
      await relaunch();
    } catch (e) {
      error = `Failed to restart: ${e}`;
    }
  }

  function handleRemindLater() {
    dispatch('later');
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }
</script>

<div class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
  <div class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-lg w-full overflow-hidden">
    <!-- Header -->
    <div class="bg-gradient-to-r from-blue-600 to-blue-700 px-6 py-8 text-white text-center">
      <div class="w-16 h-16 bg-white/20 rounded-full flex items-center justify-center mx-auto mb-4">
        <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"/>
        </svg>
      </div>
      <h2 class="text-2xl font-bold mb-1">Update Available</h2>
      <p class="text-blue-100">Version {version} is ready to install</p>
    </div>

    <!-- Content -->
    <div class="p-6 space-y-4">
      <!-- Error message -->
      {#if error}
        <div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
          <p class="text-sm text-red-700 dark:text-red-300">{error}</p>
        </div>
      {/if}

      <!-- Release Notes -->
      <div>
        <h3 class="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2 flex items-center gap-2">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
          </svg>
          What's New
        </h3>
        <div class="bg-gray-50 dark:bg-gray-900 rounded-lg p-4 max-h-48 overflow-y-auto">
          <div class="text-sm text-gray-600 dark:text-gray-400 prose prose-sm dark:prose-invert prose-p:my-1 prose-ul:my-1 prose-li:my-0">
            {#each releaseNotes.split('\n') as line}
              {#if line.startsWith('- ') || line.startsWith('* ')}
                <div class="flex items-start gap-2 py-0.5">
                  <span class="text-blue-500 mt-1">•</span>
                  <span>{line.slice(2)}</span>
                </div>
              {:else if line.startsWith('## ')}
                <h4 class="font-semibold text-gray-800 dark:text-gray-200 mt-2 mb-1">{line.slice(3)}</h4>
              {:else if line.startsWith('### ')}
                <h5 class="font-medium text-gray-700 dark:text-gray-300 mt-2 mb-1">{line.slice(4)}</h5>
              {:else if line.trim()}
                <p class="py-0.5">{line}</p>
              {/if}
            {/each}
          </div>
        </div>
      </div>

      <!-- Download Progress -->
      {#if downloading && !updateReady}
        <div class="space-y-2">
          <div class="flex justify-between text-sm">
            <span class="text-gray-600 dark:text-gray-400">Downloading update...</span>
            <span class="text-gray-900 dark:text-white font-medium">
              {#if totalBytes > 0}
                {formatBytes(downloadedBytes)} / {formatBytes(totalBytes)}
              {:else}
                {downloadProgress}%
              {/if}
            </span>
          </div>
          <div class="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-2.5 overflow-hidden">
            <div 
              class="bg-blue-600 h-2.5 rounded-full transition-all duration-300 ease-out"
              style="width: {downloadProgress}%"
            ></div>
          </div>
        </div>
      {/if}

      <!-- Update Ready Message -->
      {#if updateReady}
        <div class="p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 bg-green-100 dark:bg-green-900/30 rounded-full flex items-center justify-center">
              <svg class="w-5 h-5 text-green-600 dark:text-green-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
              </svg>
            </div>
            <div>
              <p class="font-medium text-green-800 dark:text-green-200">Update Downloaded!</p>
              <p class="text-sm text-green-600 dark:text-green-400">Restart to apply the update</p>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Actions -->
    <div class="px-6 pb-6 flex gap-3">
      {#if updateReady}
        <button
          on:click={handleRestart}
          class="flex-1 px-4 py-3 bg-green-600 hover:bg-green-700 text-white font-medium rounded-lg transition-colors flex items-center justify-center gap-2"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
          </svg>
          Restart Now
        </button>
      {:else if downloading}
        <button
          disabled
          class="flex-1 px-4 py-3 bg-blue-600 text-white font-medium rounded-lg opacity-75 cursor-not-allowed flex items-center justify-center gap-2"
        >
          <svg class="animate-spin h-5 w-5" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          Downloading...
        </button>
      {:else}
        <button
          on:click={handleRemindLater}
          class="flex-1 px-4 py-3 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 font-medium rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
        >
          Remind Me Later
        </button>
        <button
          on:click={handleDownloadAndInstall}
          class="flex-1 px-4 py-3 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors flex items-center justify-center gap-2"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
          </svg>
          Update Now
        </button>
      {/if}
    </div>
  </div>
</div>
