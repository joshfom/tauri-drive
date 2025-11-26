<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Router from 'svelte-spa-router';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import Browser from './routes/Browser.svelte';
  import Settings from './routes/Settings.svelte';
  import Transfers from './routes/Transfers.svelte';
  import SyncFolders from './routes/SyncFolders.svelte';
  import type { UploadProgress, DownloadProgress } from './lib/types';

  const routes = {
    '/': Browser,
    '/settings': Settings,
    '/transfers': Transfers,
    '/sync': SyncFolders,
  };

  interface ConnectionStatus {
    connected: boolean;
    bucket: string | null;
    error: string | null;
  }

  let connectionStatus: ConnectionStatus = { connected: false, bucket: null, error: null };
  let checkInterval: ReturnType<typeof setInterval>;
  let currentRoute = '/';
  
  // Transfer status
  let activeUploads = 0;
  let activeDownloads = 0;
  let unlistenUpload: UnlistenFn | null = null;
  let unlistenDownload: UnlistenFn | null = null;
  let uploadMap = new Map<string, boolean>();
  let downloadMap = new Map<string, boolean>();

  async function checkConnection() {
    try {
      connectionStatus = await invoke<ConnectionStatus>('check_connection');
    } catch (e) {
      connectionStatus = { connected: false, bucket: null, error: String(e) };
    }
  }

  function handleRouteChange(event: any) {
    currentRoute = event.detail?.location || '/';
  }

  onMount(async () => {
    checkConnection();
    checkInterval = setInterval(checkConnection, 30000);

    // Listen for upload/download events to track active transfers
    unlistenUpload = await listen<UploadProgress>('upload-progress', (event) => {
      const upload = event.payload;
      if (upload.status === 'completed' || upload.status === 'failed' || upload.status === 'cancelled') {
        uploadMap.delete(upload.id);
      } else {
        uploadMap.set(upload.id, true);
      }
      activeUploads = uploadMap.size;
    });

    unlistenDownload = await listen<DownloadProgress>('download-progress', (event) => {
      const download = event.payload;
      if (download.status === 'completed' || download.status === 'failed') {
        downloadMap.delete(download.id);
      } else {
        downloadMap.set(download.id, true);
      }
      activeDownloads = downloadMap.size;
    });
  });

  onDestroy(() => {
    if (checkInterval) clearInterval(checkInterval);
    if (unlistenUpload) unlistenUpload();
    if (unlistenDownload) unlistenDownload();
  });
</script>

<div class="flex h-screen bg-white dark:bg-gray-900">
  <!-- Sidebar - Flat design like Google Drive -->
  <aside class="w-64 border-r border-gray-200 dark:border-gray-800 flex flex-col">
    <!-- Logo -->
    <div class="px-5 py-4 flex items-center gap-3">
      <div class="w-10 h-10 bg-orange-500 rounded-lg flex items-center justify-center">
        <svg class="w-6 h-6 text-white" viewBox="0 0 24 24" fill="currentColor">
          <path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>
        </svg>
      </div>
      <div>
        <h1 class="text-lg font-semibold text-gray-900 dark:text-white">Cloudflare</h1>
        <p class="text-xs text-gray-500 dark:text-gray-400">Backup</p>
      </div>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 px-3 py-4 space-y-1">
      <a
        href="#/"
        class="flex items-center gap-3 px-3 py-2.5 rounded-full transition-colors {currentRoute === '/' ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300' : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'}"
        on:click={() => currentRoute = '/'}
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
        </svg>
        <span class="font-medium">My Files</span>
      </a>

      <a
        href="#/sync"
        class="flex items-center gap-3 px-3 py-2.5 rounded-full transition-colors {currentRoute === '/sync' ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300' : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'}"
        on:click={() => currentRoute = '/sync'}
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
        </svg>
        <span class="font-medium">Sync Folders</span>
      </a>

      <a
        href="#/transfers"
        class="flex items-center justify-between gap-3 px-3 py-2.5 rounded-full transition-colors {currentRoute === '/transfers' ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300' : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'}"
        on:click={() => currentRoute = '/transfers'}
      >
        <div class="flex items-center gap-3">
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4"/>
          </svg>
          <span class="font-medium">Transfers</span>
        </div>
        {#if activeUploads > 0 || activeDownloads > 0}
          <div class="flex items-center gap-1">
            {#if activeUploads > 0}
              <span class="px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 text-xs rounded-full">
                ↑{activeUploads}
              </span>
            {/if}
            {#if activeDownloads > 0}
              <span class="px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 text-xs rounded-full">
                ↓{activeDownloads}
              </span>
            {/if}
          </div>
        {/if}
      </a>

      <a
        href="#/settings"
        class="flex items-center gap-3 px-3 py-2.5 rounded-full transition-colors {currentRoute === '/settings' ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300' : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'}"
        on:click={() => currentRoute = '/settings'}
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
        </svg>
        <span class="font-medium">Settings</span>
      </a>
    </nav>

    <!-- Connection Status -->
    <div class="px-3 py-4 border-t border-gray-200 dark:border-gray-800">
      {#if connectionStatus.connected}
        <div class="flex items-center gap-3 px-3 py-2 bg-green-50 dark:bg-green-900/10 rounded-lg">
          <div class="w-2 h-2 rounded-full bg-green-500"></div>
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium text-green-700 dark:text-green-400">Connected</p>
            <p class="text-xs text-gray-500 dark:text-gray-400 truncate">{connectionStatus.bucket}</p>
          </div>
        </div>
      {:else}
        <div class="px-3 py-2 bg-gray-50 dark:bg-gray-800 rounded-lg">
          <div class="flex items-center gap-3 mb-2">
            <div class="w-2 h-2 rounded-full bg-gray-400"></div>
            <p class="text-sm font-medium text-gray-600 dark:text-gray-400">Not Connected</p>
          </div>
          <a
            href="#/settings"
            on:click={() => currentRoute = '/settings'}
            class="block w-full text-center px-3 py-2 bg-blue-600 text-white text-sm font-medium rounded-lg hover:bg-blue-700 transition-colors"
          >
            Link Account
          </a>
        </div>
      {/if}
    </div>
  </aside>

  <!-- Main content -->
  <main class="flex-1 overflow-hidden bg-gray-50 dark:bg-gray-900">
    {#if !connectionStatus.connected && currentRoute === '/'}
      <!-- Empty state when not connected -->
      <div class="flex flex-col items-center justify-center h-full text-center px-4">
        <div class="w-24 h-24 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-6">
          <svg class="w-12 h-12 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z"/>
          </svg>
        </div>
        <h2 class="text-2xl font-semibold text-gray-900 dark:text-white mb-2">Welcome to Cloudflare Backup</h2>
        <p class="text-gray-500 dark:text-gray-400 max-w-md mb-8">
          Connect your Cloudflare R2 account to start backing up your files securely to the cloud.
        </p>
        <a
          href="#/settings"
          on:click={() => currentRoute = '/settings'}
          class="px-6 py-3 bg-blue-600 text-white font-medium rounded-lg hover:bg-blue-700 transition-colors"
        >
          Link Your Account
        </a>
      </div>
    {:else}
      <Router {routes} on:routeLoaded={handleRouteChange} />
    {/if}
  </main>
</div>
