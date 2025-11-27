<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { R2Object } from '../lib/types';
  import type { FileNode } from '../lib/utils/folderParser';
  import { parseObjectsIntoFolders, getAllFilesInFolder, calculateFolderSize, getBreadcrumbs } from '../lib/utils/folderParser';
  import { uploadQueue, addToQueue, updateUploadProgress, currentBrowserPath } from '../lib/stores/uploads';
  import { formatBytes, formatDate } from '../lib/utils/formatters';
  import ConfirmDialog from '../components/ConfirmDialog.svelte';
  import CreateFolderDialog from '../components/CreateFolderDialog.svelte';
  import FilePreviewModal from '../components/FilePreviewModal.svelte';

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

  // File preview state
  let showPreviewModal = false;
  let previewFile: FileNode | null = null;

  $: breadcrumbs = currentPath ? [{ name: 'Home', path: '' }, ...getBreadcrumbs(currentPath)] : [{ name: 'Home', path: '' }];
  $: fileNodes = parseObjectsIntoFolders(objects, currentPath);
  
  // Sync currentPath to store whenever it changes
  $: currentBrowserPath.set(currentPath);

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

  function handlePreview(node: FileNode) {
    if (node.isFolder) return;
    previewFile = node;
    showPreviewModal = true;
  }

  function closePreview() {
    showPreviewModal = false;
    previewFile = null;
  }

  async function handleDownload(node: FileNode) {
    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      
      if (node.isFolder) {
        // Download folder as zip
        const savePath = await save({
          defaultPath: `${node.name}.zip`,
          filters: [{ name: 'ZIP Archive', extensions: ['zip'] }],
        });

        if (!savePath) return;

        await invoke('download_folder_as_zip', {
          folderPath: node.path,
          localPath: savePath,
        });

        alert('Folder download completed!');
      } else {
        // Download single file
        const savePath = await save({
          defaultPath: node.name,
        });

        if (!savePath) return;

        await invoke('download_file_with_progress', {
          remoteKey: node.path,
          localPath: savePath,
        });

        alert('Download completed!');
      }
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
    // Restore the last browsed path from the store
    const unsubscribe = currentBrowserPath.subscribe(path => {
      if (path && currentPath === '') {
        currentPath = path;
      }
    });
    unsubscribe(); // Only read once on mount
    
    loadObjects();
  });
</script>

<div class="flex flex-col h-full bg-white dark:bg-gray-900">
  <!-- Header - Flat design -->
  <div class="border-b border-gray-200 dark:border-gray-800 px-6 py-4">
    <div class="flex items-center justify-between">
      <!-- Breadcrumb navigation -->
      <div class="flex items-center gap-1">
        {#each breadcrumbs as crumb, i}
          {#if i > 0}
            <svg class="w-4 h-4 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
            </svg>
          {/if}
          <button
            on:click={() => navigateToFolder(crumb.path)}
            class="px-2 py-1 text-sm font-medium rounded-lg transition-colors {i === breadcrumbs.length - 1 ? 'text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-800' : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800'}"
          >
            {crumb.name}
          </button>
        {/each}
      </div>
      
      <!-- Actions -->
      <div class="flex items-center gap-2">
        <button
          on:click={loadObjects}
          class="p-2 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-colors"
          title="Refresh"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
          </svg>
        </button>
        
        <div class="h-6 w-px bg-gray-200 dark:bg-gray-700"></div>
        
        <button
          on:click={() => showCreateFolderDialog = true}
          class="flex items-center gap-2 px-3 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
          </svg>
          <span>New folder</span>
        </button>
        
        <button
          on:click={handleUploadFolder}
          disabled={uploading}
          class="flex items-center gap-2 px-3 py-2 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors disabled:opacity-50"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
          </svg>
          <span>Upload folder</span>
        </button>
        
        <button
          on:click={handleUploadFiles}
          disabled={uploading}
          class="flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors disabled:opacity-50"
        >
          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
          </svg>
          <span>{uploading ? 'Uploading...' : 'Upload'}</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Content with drag & drop -->
  <div 
    role="region"
    aria-label="File drop zone"
    class="flex-1 overflow-auto p-6"
    on:dragenter={handleDragEnter}
    on:dragleave={handleDragLeave}
    on:dragover={handleDragOver}
    on:drop={handleDrop}
  >
    <!-- Drag overlay -->
    {#if isDragging}
      <div class="fixed inset-0 bg-blue-500/10 z-40 flex items-center justify-center">
        <div class="bg-white dark:bg-gray-800 rounded-xl p-8 border-2 border-dashed border-blue-500">
          <svg class="w-12 h-12 text-blue-600 dark:text-blue-400 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
          </svg>
          <p class="mt-4 text-lg font-medium text-gray-900 dark:text-white">Drop files here to upload</p>
        </div>
      </div>
    {/if}

    {#if loading}
      <div class="flex items-center justify-center h-64">
        <div class="text-center">
          <div class="animate-spin rounded-full h-10 w-10 border-2 border-gray-200 border-t-blue-600 mx-auto"></div>
          <p class="mt-3 text-sm text-gray-500 dark:text-gray-400">Loading files...</p>
        </div>
      </div>
    {:else if error}
      <div class="max-w-md mx-auto mt-8">
        <div class="bg-red-50 dark:bg-red-900/10 border border-red-200 dark:border-red-800 rounded-lg p-4">
          <div class="flex items-start gap-3">
            <svg class="w-5 h-5 text-red-600 dark:text-red-400 mt-0.5 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
            </svg>
            <div>
              <h3 class="text-sm font-medium text-red-800 dark:text-red-200">Error</h3>
              <p class="text-sm text-red-700 dark:text-red-300 mt-1">{error}</p>
            </div>
          </div>
        </div>
      </div>
    {:else if fileNodes.length === 0}
      <div class="flex items-center justify-center h-64">
        <div class="text-center">
          <div class="mx-auto w-20 h-20 bg-gray-100 dark:bg-gray-800 rounded-full flex items-center justify-center">
            <svg class="w-10 h-10 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
            </svg>
          </div>
          <h3 class="mt-4 text-lg font-medium text-gray-900 dark:text-white">This folder is empty</h3>
          <p class="mt-1 text-sm text-gray-500 dark:text-gray-400">Upload files or create a folder to get started</p>
          <div class="mt-4 flex gap-2 justify-center">
            <button
              on:click={handleUploadFiles}
              class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
            >
              Upload files
            </button>
            <button
              on:click={handleUploadFolder}
              class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-800 rounded-lg transition-colors"
            >
              Upload folder
            </button>
          </div>
        </div>
      </div>
    {:else}
      <!-- Grid view like Google Drive -->
      <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-3">
        {#each fileNodes as node}
          <div
            role="button"
            tabindex="0"
            class="group relative bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl p-3 hover:border-blue-500 dark:hover:border-blue-500 transition-colors cursor-pointer text-left w-full focus:outline-none focus:ring-2 focus:ring-blue-500"
            on:dblclick={() => node.isFolder ? navigateToFolder(node.path) : handlePreview(node)}
            on:keydown={(e) => e.key === 'Enter' && (node.isFolder ? navigateToFolder(node.path) : handlePreview(node))}
          >
            <!-- File/Folder icon -->
            <div class="aspect-square rounded-lg bg-gray-50 dark:bg-gray-900 flex items-center justify-center mb-3">
              {#if node.isFolder}
                <svg class="w-16 h-16 text-amber-500" fill="currentColor" viewBox="0 0 24 24">
                  <path d="M10 4H4a2 2 0 00-2 2v12a2 2 0 002 2h16a2 2 0 002-2V8a2 2 0 00-2-2h-8l-2-2z"/>
                </svg>
              {:else}
                {@const ext = node.name.split('.').pop()?.toLowerCase() || ''}
                {#if ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp'].includes(ext)}
                  <svg class="w-16 h-16 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                  </svg>
                {:else if ['mp4', 'mov', 'avi', 'mkv', 'webm'].includes(ext)}
                  <svg class="w-16 h-16 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/>
                  </svg>
                {:else if ['mp3', 'wav', 'flac', 'aac', 'ogg'].includes(ext)}
                  <svg class="w-16 h-16 text-pink-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"/>
                  </svg>
                {:else if ['pdf'].includes(ext)}
                  <svg class="w-16 h-16 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z"/>
                  </svg>
                {:else if ['doc', 'docx', 'txt', 'rtf'].includes(ext)}
                  <svg class="w-16 h-16 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                  </svg>
                {:else if ['xls', 'xlsx', 'csv'].includes(ext)}
                  <svg class="w-16 h-16 text-emerald-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/>
                  </svg>
                {:else if ['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)}
                  <svg class="w-16 h-16 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4"/>
                  </svg>
                {:else}
                  <svg class="w-16 h-16 text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                  </svg>
                {/if}
              {/if}
            </div>
            
            <!-- File info -->
            <div class="min-w-0">
              <p class="text-sm font-medium text-gray-900 dark:text-white truncate" title={node.name}>
                {node.name}
              </p>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                {node.isFolder ? `${node.children?.length || 0} items` : formatBytes(node.size)}
              </p>
            </div>
            
            <!-- Action menu on hover -->
            <div class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
              <div class="flex items-center gap-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-1">
                {#if !node.isFolder}
                  <button
                    on:click|stopPropagation={() => handlePreview(node)}
                    class="p-1.5 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
                    title="Preview"
                  >
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z"/>
                    </svg>
                  </button>
                {/if}
                <button
                  on:click|stopPropagation={() => handleDownload(node)}
                  class="p-1.5 text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
                  title={node.isFolder ? 'Download as ZIP' : 'Download'}
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
                  </svg>
                </button>
                <button
                  on:click|stopPropagation={() => handleDelete(node)}
                  class="p-1.5 text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded"
                  title="Delete"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
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

<!-- File Preview Modal -->
<FilePreviewModal
  bind:isOpen={showPreviewModal}
  file={previewFile}
  onClose={closePreview}
/>
