<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';

  interface SyncFolder {
    id: number;
    local_path: string;
    remote_path: string;
    enabled: boolean;
    last_sync: string | null;
  }

  interface SyncStatus {
    folderId: number;
    status: 'syncing' | 'completed' | 'error';
    currentFile?: string;
    progress: number;
    error?: string;
  }

  let syncFolders: SyncFolder[] = [];
  let loading = false;
  let error = '';
  let showAddDialog = false;
  let newFolderPath = '';
  let newRemotePath = '';
  let unlistenSync: UnlistenFn | null = null;

  async function loadSyncFolders() {
    loading = true;
    try {
      syncFolders = await invoke<SyncFolder[]>('get_sync_folders');
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleAddFolder() {
    try {
      const selected = await open({
        multiple: false,
        directory: true,
      });

      if (!selected || Array.isArray(selected)) return;

      newFolderPath = selected;
      // Default remote path to folder name
      const folderName = selected.split('/').pop() || selected.split('\\').pop() || 'backup';
      newRemotePath = folderName;
      showAddDialog = true;
    } catch (e) {
      error = String(e);
    }
  }

  async function confirmAddFolder() {
    if (!newFolderPath || !newRemotePath) return;

    try {
      await invoke('add_sync_folder', {
        localPath: newFolderPath,
        remotePath: newRemotePath,
      });
      showAddDialog = false;
      newFolderPath = '';
      newRemotePath = '';
      await loadSyncFolders();
    } catch (e) {
      error = String(e);
    }
  }

  async function handleRemoveFolder(id: number) {
    try {
      await invoke('remove_sync_folder', { folderId: id });
      await loadSyncFolders();
    } catch (e) {
      error = String(e);
    }
  }

  async function handleToggleFolder(folder: SyncFolder) {
    try {
      await invoke('toggle_sync_folder', { 
        folderId: folder.id, 
        enabled: !folder.enabled 
      });
      await loadSyncFolders();
    } catch (e) {
      error = String(e);
    }
  }

  async function handleSyncNow(id: number) {
    try {
      await invoke('sync_folder_now', { folderId: id });
    } catch (e) {
      error = String(e);
    }
  }

  onMount(async () => {
    await loadSyncFolders();
    
    // Listen for sync status updates
    unlistenSync = await listen<SyncStatus>('sync-status', (event) => {
      const status = event.payload;
      syncFolders = syncFolders.map(f => 
        f.id === status.folderId 
          ? { ...f, status: status.status === 'completed' ? 'idle' : status.status }
          : f
      );
    });
  });

  onDestroy(() => {
    if (unlistenSync) unlistenSync();
  });
</script>

<div class="flex flex-col h-full bg-gray-50 dark:bg-gray-900">
  <!-- Header -->
  <div class="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-xl font-semibold text-gray-900 dark:text-white">Sync Folders</h2>
        <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Automatically backup folders to Cloudflare R2
        </p>
      </div>
      <button
        on:click={handleAddFolder}
        class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors flex items-center gap-2"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
        </svg>
        Add Folder
      </button>
    </div>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-auto p-6">
    {#if error}
      <div class="mb-4 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
        <p class="text-red-700 dark:text-red-300">{error}</p>
      </div>
    {/if}

    {#if loading}
      <div class="flex items-center justify-center h-64">
        <div class="animate-spin rounded-full h-8 w-8 border-2 border-blue-600 border-t-transparent"></div>
      </div>
    {:else if syncFolders.length === 0}
      <div class="flex flex-col items-center justify-center h-64 text-center">
        <div class="w-16 h-16 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center mb-4">
          <svg class="w-8 h-8 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
          </svg>
        </div>
        <h3 class="text-lg font-medium text-gray-900 dark:text-white mb-2">No sync folders</h3>
        <p class="text-gray-500 dark:text-gray-400 mb-4 max-w-md">
          Add a folder to automatically backup its contents to Cloudflare R2. 
          Files will be uploaded whenever they change.
        </p>
        <button
          on:click={handleAddFolder}
          class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        >
          Add Your First Folder
        </button>
      </div>
    {:else}
      <div class="space-y-4">
        {#each syncFolders as folder}
          <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4">
            <div class="flex items-start justify-between">
              <div class="flex items-start gap-4">
                <div class="w-10 h-10 bg-blue-100 dark:bg-blue-900/30 rounded-lg flex items-center justify-center flex-shrink-0">
                  <svg class="w-5 h-5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                  </svg>
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <h3 class="font-medium text-gray-900 dark:text-white truncate">
                      {folder.local_path.split('/').pop() || folder.local_path}
                    </h3>
                    {#if !folder.enabled}
                      <span class="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 text-xs rounded-full">
                        Paused
                      </span>
                    {:else}
                      <span class="px-2 py-0.5 bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 text-xs rounded-full">
                        Active
                      </span>
                    {/if}
                  </div>
                  <p class="text-sm text-gray-500 dark:text-gray-400 truncate mt-1" title={folder.local_path}>
                    {folder.local_path}
                  </p>
                  <div class="flex items-center gap-4 mt-2 text-xs text-gray-400 dark:text-gray-500">
                    <span>→ /{folder.remote_path}</span>
                    {#if folder.last_sync}
                      <span>Last sync: {folder.last_sync}</span>
                    {/if}
                  </div>
                </div>
              </div>
              <div class="flex items-center gap-2">
                <button
                  on:click={() => handleToggleFolder(folder)}
                  class="p-2 text-gray-500 hover:text-gray-700 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                  title={folder.enabled ? 'Pause sync' : 'Resume sync'}
                >
                  {#if folder.enabled}
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                  {:else}
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z"/>
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                  {/if}
                </button>
                <button
                  on:click={() => handleRemoveFolder(folder.id)}
                  class="p-2 text-gray-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                  title="Remove folder"
                >
                  <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                  </svg>
                </button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<!-- Add Folder Dialog -->
{#if showAddDialog}
  <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 w-full max-w-md mx-4">
      <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Add Sync Folder</h3>
      
      <div class="space-y-4">
        <div>
          <label for="local-folder-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Local Folder
          </label>
          <input
            id="local-folder-input"
            type="text"
            value={newFolderPath}
            readonly
            class="w-full px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg text-gray-900 dark:text-white"
          />
        </div>
        
        <div>
          <label for="remote-path-input" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Remote Path (in R2)
          </label>
          <div class="flex items-center">
            <span class="text-gray-500 dark:text-gray-400 mr-1">/</span>
            <input
              id="remote-path-input"
              type="text"
              bind:value={newRemotePath}
              placeholder="backup/folder"
              class="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
            />
          </div>
        </div>

        <div class="bg-blue-50 dark:bg-blue-900/20 rounded-lg p-3">
          <p class="text-sm text-blue-700 dark:text-blue-300">
            <strong>Note:</strong> This is a one-way backup. Files added or modified in this folder will be automatically uploaded to R2.
          </p>
        </div>
      </div>
      
      <div class="flex justify-end gap-3 mt-6">
        <button
          on:click={() => showAddDialog = false}
          class="px-4 py-2 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
        >
          Cancel
        </button>
        <button
          on:click={confirmAddFolder}
          class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
        >
          Add Folder
        </button>
      </div>
    </div>
  </div>
{/if}
