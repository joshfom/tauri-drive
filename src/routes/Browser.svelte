<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { R2Object } from '../lib/types';
  import type { FileNode } from '../lib/utils/folderParser';
  import { parseObjectsIntoFolders, getAllFilesInFolder, calculateFolderSize, getBreadcrumbs } from '../lib/utils/folderParser';
  import { uploadQueue, addToQueue, updateUploadProgress } from '../lib/stores/uploads';
  import { formatBytes, formatDate } from '../lib/utils/formatters';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import CreateFolderDialog from '../components/CreateFolderDialog.svelte';

  let objects: R2Object[] = [];
  let fileNodes: FileNode[] = [];
  let currentPath = '';
  let breadcrumbs: { name: string; path: string }[] = [];
  let loading = false;
  let error = '';
  let uploading = false;
  let isDragging = false;

  // Confirmation dialog state
  let showConfirmDialog = false;
  let confirmTitle = '';
  let confirmMessage = '';
  let confirmAction: (() => void) | null = null;

  // Create folder dialog state
  let showCreateFolderDialog = false;

  $: breadcrumbs = currentPath ? [{ name: 'Home', path: '' }, ...getBreadcrumbs(currentPath)] : [{ name: 'Home', path: '' }];
  $: fileNodes = parseObjectsIntoFolders(objects, currentPath);

  async function loadObjects() {
    loading = true;
    error = '';
    try {
      const prefix = currentPath ? (currentPath.endsWith('/') ? currentPath : `${currentPath}/`) : null;
      objects = await invoke<R2Object[]>('list_objects', { prefix });
    } catch (e) {
      error = e as string;
    } finally {
      loading = false;
    }
  }

  function navigateToFolder(path: string) {
    currentPath = path;
    loadObjects();
  }

  async function handleUploadFiles() {
    try {
      const selected = await open({
        multiple: true,
        directory: false,
      });

      if (!selected) return;

      const files = Array.isArray(selected) ? selected : [selected];
      await uploadFiles(files);
    } catch (e) {
      error = `Upload failed: ${e}`;
    }
  }

  async function handleUploadFolder() {
    try {
      const selected = await open({
        multiple: false,
        directory: true,
      });

      if (!selected || Array.isArray(selected)) return;

      await uploadFolder(selected);
    } catch (e) {
      error = `Folder upload failed: ${e}`;
    }
  }

  async function uploadFiles(filePaths: string[]) {
    uploading = true;
    
    for (const filePath of filePaths) {
      const fileName = filePath.split('/').pop() || filePath.split('\\').pop() || 'unknown';
      const remotePath = currentPath ? `${currentPath}/${fileName}` : fileName;
      
      try {
        // The Rust command will emit upload-progress events with the upload ID
        // The UploadQueue component is already listening for these events
        await invoke<string>('upload_file_with_progress', {
          localPath: filePath,
          remoteKey: remotePath,
        });
      } catch (e) {
        console.error('Upload failed:', e);
        error = `Failed to upload ${fileName}: ${e}`;
      }
    }
    
    uploading = false;
    await loadObjects();
  }

  async function uploadFolder(folderPath: string) {
    uploading = true;
    error = '';
    
    try {
      // Get all files in the folder recursively
      const allFiles = await invoke<string[]>('list_directory', { directoryPath: folderPath });
      
      if (allFiles.length === 0) {
        error = 'Folder is empty';
        uploading = false;
        return;
      }

      // Get folder name from path
      const folderName = folderPath.split('/').pop() || folderPath.split('\\').pop() || 'folder';
      
      // Upload all files maintaining folder structure
      for (const filePath of allFiles) {
        // Get relative path from the selected folder
        const relativePath = filePath.replace(folderPath, '').replace(/^\//, '').replace(/^\\/, '');
        
        // Construct remote path
        const remotePath = currentPath 
          ? `${currentPath}/${folderName}/${relativePath}`
          : `${folderName}/${relativePath}`;
        
        try {
          await invoke<string>('upload_file_with_progress', {
            localPath: filePath,
            remoteKey: remotePath,
          });
        } catch (e) {
          console.error(`Failed to upload ${filePath}:`, e);
          error = `Failed to upload some files: ${e}`;
        }
      }
    } catch (e) {
      error = `Folder upload failed: ${e}`;
    } finally {
      uploading = false;
      await loadObjects();
    }
  }

  async function handleCreateFolder(folderName: string) {
    try {
      const folderPath = currentPath ? `${currentPath}/${folderName}` : folderName;
      await invoke('create_folder', { folderPath });
      await loadObjects();
    } catch (e) {
      error = `Failed to create folder: ${e}`;
    }
  }

  async function handleDownload(node: FileNode) {
    if (node.isFolder) {
      error = 'Folder download coming soon!';
      return;
    }

    try {
      const savePath = await open({
        directory: false,
        multiple: false,
        defaultPath: node.name,
      });

      if (!savePath || Array.isArray(savePath)) return;

      await invoke('download_file', {
        remoteKey: node.path,
        localPath: savePath,
      });

      alert('Download completed!');
    } catch (e) {
      error = `Download failed: ${e}`;
    }
  }

  async function handleDelete(node: FileNode) {
    const itemType = node.isFolder ? 'folder' : 'file';
    confirmTitle = `Delete ${itemType}?`;
    confirmMessage = node.isFolder 
      ? `Are you sure you want to delete folder "${node.name}" and all its contents? This action cannot be undone.`
      : `Are you sure you want to delete "${node.name}"? This action cannot be undone.`;
    
    confirmAction = async () => {
      try {
        if (node.isFolder) {
          // Get all files in folder and delete them
          const files = getAllFilesInFolder(node);
          for (const filePath of files) {
            await invoke('delete_file', { remoteKey: filePath });
          }
          // Also delete the folder marker if it exists
          try {
            await invoke('delete_file', { remoteKey: node.path + '/' });
          } catch (e) {
            // Folder marker might not exist, that's ok
          }
        } else {
          await invoke('delete_file', { remoteKey: node.path });
        }
        await loadObjects();
      } catch (e) {
        error = `Delete failed: ${e}`;
      }
    };
    
    showConfirmDialog = true;
  }

  // Drag and drop handlers
  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    isDragging = true;
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    if (e.currentTarget === e.target) {
      isDragging = false;
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;

    if (!e.dataTransfer?.files) return;

    const filePaths: string[] = [];
    for (let i = 0; i < e.dataTransfer.files.length; i++) {
      const file = e.dataTransfer.files[i];
      // @ts-ignore - Tauri provides path property
      if (file.path) {
        // @ts-ignore
        filePaths.push(file.path);
      }
    }

    if (filePaths.length > 0) {
      await uploadFiles(filePaths);
    }
  }

  onMount(() => {
    loadObjects();
  });
</script>

<div class="flex flex-col h-full bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900 dark:to-gray-800">
  <!-- Header -->
  <div class="bg-white/80 dark:bg-gray-800/80 backdrop-blur-sm border-b border-gray-200/50 dark:border-gray-700/50 px-8 py-6 shadow-sm">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-3xl font-bold text-gray-900 dark:text-white">File Browser</h2>
        <!-- Breadcrumb navigation -->
        <div class="flex items-center space-x-2 mt-2">
          {#each breadcrumbs as crumb, i}
            {#if i > 0}
              <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
              </svg>
            {/if}
            <button
              on:click={() => navigateToFolder(crumb.path)}
              class="text-sm font-medium {i === breadcrumbs.length - 1 ? 'text-blue-600 dark:text-blue-400' : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'}"
            >
              {crumb.name}
            </button>
          {/each}
        </div>
      </div>
      <div class="flex space-x-3">
        <button
          on:click={loadObjects}
          class="px-5 py-2.5 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 border border-gray-300 dark:border-gray-600 rounded-xl hover:bg-gray-50 dark:hover:bg-gray-600 transition-all duration-200 flex items-center space-x-2 shadow-sm hover:shadow"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
          </svg>
          <span>Refresh</span>
        </button>
        <button
          on:click={() => showCreateFolderDialog = true}
          class="px-5 py-2.5 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 border border-gray-300 dark:border-gray-600 rounded-xl hover:bg-gray-50 dark:hover:bg-gray-600 transition-all duration-200 flex items-center space-x-2 shadow-sm hover:shadow"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
          </svg>
          <span>New Folder</span>
        </button>
        <button
          on:click={handleUploadFolder}
          disabled={uploading}
          class="px-5 py-2.5 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 border border-gray-300 dark:border-gray-600 rounded-xl hover:bg-gray-50 dark:hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200 flex items-center space-x-2 shadow-sm hover:shadow"
        >
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
          </svg>
          <span>Upload Folder</span>
        </button>
        <button
          on:click={handleUploadFiles}
          disabled={uploading}
          class="px-6 py-2.5 bg-gradient-to-r from-blue-600 to-blue-700 text-white rounded-xl hover:from-blue-700 hover:to-blue-800 disabled:from-gray-400 disabled:to-gray-500 disabled:cursor-not-allowed transition-all duration-200 flex items-center space-x-2 shadow-lg hover:shadow-xl transform hover:-translate-y-0.5"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
          </svg>
          <span>{uploading ? 'Uploading...' : 'Upload Files'}</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Content with drag & drop -->
  <div 
    role="region"
    aria-label="File drop zone"
    class="flex-1 overflow-auto p-8"
    on:dragenter={handleDragEnter}
    on:dragleave={handleDragLeave}
    on:dragover={handleDragOver}
    on:drop={handleDrop}
  >
    <!-- Drag overlay -->
    {#if isDragging}
      <div class="fixed inset-0 bg-blue-500/20 backdrop-blur-sm z-40 flex items-center justify-center">
        <div class="bg-white dark:bg-gray-800 rounded-2xl p-8 shadow-2xl border-4 border-dashed border-blue-500">
          <svg class="w-16 h-16 text-blue-600 dark:text-blue-400 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
          </svg>
          <p class="mt-4 text-xl font-semibold text-gray-900 dark:text-white">Drop files here to upload</p>
        </div>
      </div>
    {/if}

    {#if loading}
      <div class="flex items-center justify-center h-96">
        <div class="text-center">
          <div class="animate-spin rounded-full h-16 w-16 border-4 border-blue-200 border-t-blue-600 mx-auto"></div>
          <p class="mt-4 text-gray-600 dark:text-gray-400">Loading files...</p>
        </div>
      </div>
    {:else if error}
      <div class="max-w-2xl mx-auto mt-8">
        <div class="bg-gradient-to-r from-red-50 to-red-100 dark:from-red-900/20 dark:to-red-800/20 border-l-4 border-red-500 rounded-xl p-6 shadow-lg">
          <div class="flex items-start">
            <svg class="w-6 h-6 text-red-600 dark:text-red-400 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
            </svg>
            <div class="ml-4">
              <h3 class="text-lg font-semibold text-red-800 dark:text-red-200">Error</h3>
              <p class="text-red-700 dark:text-red-300 mt-2">{error}</p>
            </div>
          </div>
        </div>
      </div>
    {:else if fileNodes.length === 0}
      <div class="flex items-center justify-center h-96">
        <div class="text-center">
          <div class="mx-auto w-32 h-32 bg-gradient-to-br from-blue-100 to-blue-200 dark:from-blue-900/30 dark:to-blue-800/30 rounded-3xl flex items-center justify-center shadow-xl">
            <svg class="w-16 h-16 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
            </svg>
          </div>
          <h3 class="mt-6 text-xl font-semibold text-gray-900 dark:text-white">This folder is empty</h3>
          <p class="mt-2 text-gray-500 dark:text-gray-400">Upload files or folders to get started</p>
          <div class="mt-6 flex gap-3 justify-center">
            <button
              on:click={handleUploadFiles}
              class="px-6 py-3 bg-gradient-to-r from-blue-600 to-blue-700 text-white rounded-xl hover:from-blue-700 hover:to-blue-800 transition-all duration-200 shadow-lg hover:shadow-xl transform hover:-translate-y-0.5"
            >
              Upload Files
            </button>
            <button
              on:click={handleUploadFolder}
              class="px-6 py-3 bg-gradient-to-r from-purple-600 to-purple-700 text-white rounded-xl hover:from-purple-700 hover:to-purple-800 transition-all duration-200 shadow-lg hover:shadow-xl transform hover:-translate-y-0.5"
            >
              Upload Folder
            </button>
          </div>
        </div>
      </div>
    {:else}
      <div class="bg-white/80 dark:bg-gray-800/80 backdrop-blur-sm shadow-xl rounded-2xl overflow-hidden border border-gray-200/50 dark:border-gray-700/50">
        <div class="overflow-x-auto">
          <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
            <thead class="bg-gradient-to-r from-gray-50 to-gray-100 dark:from-gray-700 dark:to-gray-800">
              <tr>
                <th class="px-6 py-4 text-left text-xs font-bold text-gray-700 dark:text-gray-300 uppercase tracking-wider">
                  Name
                </th>
                <th class="px-6 py-4 text-left text-xs font-bold text-gray-700 dark:text-gray-300 uppercase tracking-wider">
                  Size
                </th>
                <th class="px-6 py-4 text-left text-xs font-bold text-gray-700 dark:text-gray-300 uppercase tracking-wider">
                  Last Modified
                </th>
                <th class="px-6 py-4 text-right text-xs font-bold text-gray-700 dark:text-gray-300 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200/50 dark:divide-gray-700/50">
              {#each fileNodes as node}
                <tr 
                  class="hover:bg-blue-50/50 dark:hover:bg-blue-900/10 transition-colors duration-150 {node.isFolder ? 'cursor-pointer' : ''}"
                  on:dblclick={() => node.isFolder && navigateToFolder(node.path)}
                >
                  <td class="px-6 py-4 whitespace-nowrap">
                    <div class="flex items-center">
                      <div class="w-10 h-10 bg-gradient-to-br {node.isFolder ? 'from-yellow-100 to-amber-200 dark:from-yellow-900/40 dark:to-amber-800/40' : 'from-blue-100 to-blue-200 dark:from-blue-900/40 dark:to-blue-800/40'} rounded-xl flex items-center justify-center mr-3">
                        {#if node.isFolder}
                          <svg class="h-5 w-5 text-yellow-600 dark:text-yellow-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                          </svg>
                        {:else}
                          <svg class="h-5 w-5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                          </svg>
                        {/if}
                      </div>
                      <span class="text-sm font-medium text-gray-900 dark:text-white">{node.name}</span>
                    </div>
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap">
                    <span class="text-sm text-gray-600 dark:text-gray-400 font-medium">
                      {node.isFolder ? `${node.children?.length || 0} items` : formatBytes(node.size)}
                    </span>
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap">
                    <span class="text-sm text-gray-600 dark:text-gray-400">
                      {formatDate(node.lastModified)}
                    </span>
                  </td>
                  <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                    {#if !node.isFolder}
                      <button
                        on:click={() => handleDownload(node)}
                        class="inline-flex items-center px-3 py-1.5 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 rounded-lg hover:bg-blue-200 dark:hover:bg-blue-900/50 transition-colors duration-150 mr-2"
                      >
                        <svg class="w-4 h-4 mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                        </svg>
                        Download
                      </button>
                    {/if}
                    <button
                      on:click={() => handleDelete(node)}
                      class="inline-flex items-center px-3 py-1.5 bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400 rounded-lg hover:bg-red-200 dark:hover:bg-red-900/50 transition-colors duration-150"
                    >
                      <svg class="w-4 h-4 mr-1.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                      </svg>
                      Delete
                    </button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {/if}
  </div>
</div>

<!-- Confirmation Dialog -->
<ConfirmDialog
  bind:isOpen={showConfirmDialog}
  title={confirmTitle}
  message={confirmMessage}
  confirmText="Delete"
  cancelText="Cancel"
  dangerous={true}
  onConfirm={() => confirmAction && confirmAction()}
  onCancel={() => {}}
/>

<!-- Create Folder Dialog -->
<CreateFolderDialog
  bind:isOpen={showCreateFolderDialog}
  currentPath={currentPath}
  onConfirm={handleCreateFolder}
  onCancel={() => {}}
/>
