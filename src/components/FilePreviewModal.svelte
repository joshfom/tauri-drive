<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { formatBytes } from '../lib/utils/formatters';
  import type { FileNode } from '../lib/utils/folderParser';

  export let isOpen = false;
  export let file: FileNode | null = null;
  export let onClose: () => void = () => {};

  let loading = false;
  let error = '';
  let previewUrl = '';
  let textContent = '';
  let tempFilePath = '';

  // File type detection
  $: fileType = file ? getFileType(file.name) : 'unknown';
  $: isPreviewable = ['image', 'video', 'audio', 'text', 'pdf'].includes(fileType);

  function getFileType(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase() || '';
    
    // Images
    if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp', 'ico'].includes(ext)) {
      return 'image';
    }
    // Videos
    if (['mp4', 'webm', 'mov', 'avi', 'mkv', 'm4v'].includes(ext)) {
      return 'video';
    }
    // Audio
    if (['mp3', 'wav', 'ogg', 'flac', 'm4a', 'aac'].includes(ext)) {
      return 'audio';
    }
    // PDF
    if (ext === 'pdf') {
      return 'pdf';
    }
    // Text files
    if (['txt', 'md', 'json', 'js', 'ts', 'css', 'html', 'xml', 'yaml', 'yml', 'toml', 'ini', 'log', 'sh', 'py', 'rs', 'go', 'java', 'c', 'cpp', 'h', 'hpp', 'svelte', 'vue', 'jsx', 'tsx'].includes(ext)) {
      return 'text';
    }
    
    return 'unknown';
  }

  function getLanguage(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase() || '';
    const langMap: Record<string, string> = {
      'js': 'javascript',
      'ts': 'typescript',
      'jsx': 'javascript',
      'tsx': 'typescript',
      'py': 'python',
      'rs': 'rust',
      'go': 'go',
      'java': 'java',
      'c': 'c',
      'cpp': 'cpp',
      'h': 'c',
      'hpp': 'cpp',
      'css': 'css',
      'html': 'html',
      'xml': 'xml',
      'json': 'json',
      'yaml': 'yaml',
      'yml': 'yaml',
      'md': 'markdown',
      'sh': 'bash',
      'svelte': 'svelte',
      'vue': 'vue',
    };
    return langMap[ext] || 'plaintext';
  }

  async function loadPreview() {
    if (!file || !isOpen) return;

    loading = true;
    error = '';
    previewUrl = '';
    textContent = '';

    try {
      // Download to temp file for preview
      const tempDir = await invoke<string>('get_temp_dir');
      tempFilePath = `${tempDir}/preview_${Date.now()}_${file.name}`;
      
      await invoke('download_file_with_progress', {
        remoteKey: file.path,
        localPath: tempFilePath,
      });

      if (fileType === 'image' || fileType === 'video' || fileType === 'audio' || fileType === 'pdf') {
        // Convert to asset URL for Tauri
        previewUrl = convertFileSrc(tempFilePath);
      } else if (fileType === 'text') {
        // Read text content
        textContent = await invoke<string>('read_text_file', { path: tempFilePath });
      }
    } catch (e) {
      error = `Failed to load preview: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function cleanup() {
    if (tempFilePath) {
      try {
        await invoke('delete_temp_file', { path: tempFilePath });
      } catch (e) {
        // Ignore cleanup errors
      }
      tempFilePath = '';
    }
    previewUrl = '';
    textContent = '';
  }

  function handleClose() {
    cleanup();
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleClose();
    }
  }

  // React to file changes
  $: if (file && isOpen) {
    loadPreview();
  }

  $: if (!isOpen) {
    cleanup();
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen && file}
  <!-- Backdrop -->
  <div 
    class="fixed inset-0 bg-black/70 backdrop-blur-sm z-50 flex items-center justify-center p-4"
    on:click={handleClose}
    role="dialog"
    aria-modal="true"
    aria-labelledby="preview-title"
  >
    <!-- Modal -->
    <div 
      class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-5xl w-full max-h-[90vh] flex flex-col overflow-hidden"
      on:click|stopPropagation
    >
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-center space-x-3">
          <div class="w-10 h-10 bg-gradient-to-br from-blue-100 to-blue-200 dark:from-blue-900/40 dark:to-blue-800/40 rounded-xl flex items-center justify-center">
            {#if fileType === 'image'}
              <svg class="w-5 h-5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"/>
              </svg>
            {:else if fileType === 'video'}
              <svg class="w-5 h-5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 10l4.553-2.276A1 1 0 0121 8.618v6.764a1 1 0 01-1.447.894L15 14M5 18h8a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z"/>
              </svg>
            {:else if fileType === 'audio'}
              <svg class="w-5 h-5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"/>
              </svg>
            {:else}
              <svg class="w-5 h-5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
              </svg>
            {/if}
          </div>
          <div>
            <h3 id="preview-title" class="text-lg font-semibold text-gray-900 dark:text-white">{file.name}</h3>
            <p class="text-sm text-gray-500 dark:text-gray-400">{formatBytes(file.size)}</p>
          </div>
        </div>
        <button
          on:click={handleClose}
          class="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
        >
          <svg class="w-6 h-6 text-gray-500 dark:text-gray-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
          </svg>
        </button>
      </div>

      <!-- Content -->
      <div class="flex-1 overflow-auto p-6 bg-gray-50 dark:bg-gray-900">
        {#if loading}
          <div class="flex items-center justify-center h-64">
            <div class="text-center">
              <div class="animate-spin rounded-full h-12 w-12 border-4 border-blue-200 border-t-blue-600 mx-auto"></div>
              <p class="mt-4 text-gray-600 dark:text-gray-400">Loading preview...</p>
            </div>
          </div>
        {:else if error}
          <div class="flex items-center justify-center h-64">
            <div class="text-center">
              <svg class="w-16 h-16 text-red-400 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
              </svg>
              <p class="mt-4 text-red-600 dark:text-red-400">{error}</p>
            </div>
          </div>
        {:else if !isPreviewable}
          <div class="flex items-center justify-center h-64">
            <div class="text-center">
              <svg class="w-16 h-16 text-gray-400 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
              </svg>
              <p class="mt-4 text-gray-600 dark:text-gray-400">Preview not available for this file type</p>
              <p class="text-sm text-gray-500 dark:text-gray-500">Download the file to view it</p>
            </div>
          </div>
        {:else if fileType === 'image' && previewUrl}
          <div class="flex items-center justify-center">
            <img 
              src={previewUrl} 
              alt={file.name}
              class="max-w-full max-h-[70vh] object-contain rounded-lg shadow-lg"
            />
          </div>
        {:else if fileType === 'video' && previewUrl}
          <div class="flex items-center justify-center">
            <video 
              src={previewUrl} 
              controls 
              class="max-w-full max-h-[70vh] rounded-lg shadow-lg"
            >
              <track kind="captions" />
            </video>
          </div>
        {:else if fileType === 'audio' && previewUrl}
          <div class="flex items-center justify-center py-8">
            <audio src={previewUrl} controls class="w-full max-w-md">
              <track kind="captions" />
            </audio>
          </div>
        {:else if fileType === 'pdf' && previewUrl}
          <iframe
            src={previewUrl}
            title={file.name}
            class="w-full h-[70vh] rounded-lg shadow-lg"
          />
        {:else if fileType === 'text' && textContent}
          <div class="bg-gray-900 rounded-lg shadow-lg overflow-auto max-h-[70vh]">
            <pre class="p-4 text-sm text-gray-100 font-mono whitespace-pre-wrap break-words"><code>{textContent}</code></pre>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
