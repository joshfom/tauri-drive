<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let isOpen = false;
  export let currentName = '';
  export let isFolder = false;
  export let onConfirm: (newName: string) => void = () => {};
  export let onCancel: () => void = () => {};

  const dispatch = createEventDispatcher();
  
  let newName = '';
  let inputElement: HTMLInputElement;

  $: if (isOpen) {
    newName = currentName;
    // Focus and select the name (without extension for files)
    setTimeout(() => {
      if (inputElement) {
        inputElement.focus();
        if (!isFolder && currentName.includes('.')) {
          const lastDot = currentName.lastIndexOf('.');
          inputElement.setSelectionRange(0, lastDot);
        } else {
          inputElement.select();
        }
      }
    }, 50);
  }

  function handleConfirm() {
    if (!newName.trim() || newName === currentName) {
      handleCancel();
      return;
    }
    onConfirm(newName.trim());
    isOpen = false;
  }

  function handleCancel() {
    onCancel();
    isOpen = false;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      handleConfirm();
    } else if (e.key === 'Escape') {
      handleCancel();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleCancel();
    }
  }
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="rename-title"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
    on:click={handleBackdropClick}
    on:keydown={handleKeydown}
  >
    <div class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl w-full max-w-md mx-4 overflow-hidden">
      <!-- Header -->
      <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
        <h3 id="rename-title" class="text-lg font-semibold text-gray-900 dark:text-white">
          Rename {isFolder ? 'Folder' : 'File'}
        </h3>
      </div>
      
      <!-- Content -->
      <div class="px-6 py-4">
        <label for="new-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          New name
        </label>
        <input
          bind:this={inputElement}
          bind:value={newName}
          id="new-name"
          type="text"
          class="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg 
                 bg-white dark:bg-gray-700 text-gray-900 dark:text-white
                 focus:ring-2 focus:ring-blue-500 focus:border-blue-500
                 placeholder-gray-400 dark:placeholder-gray-500"
          placeholder="Enter new name"
        />
      </div>
      
      <!-- Footer -->
      <div class="px-6 py-4 bg-gray-50 dark:bg-gray-900/50 flex justify-end gap-3">
        <button
          on:click={handleCancel}
          class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 
                 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
        >
          Cancel
        </button>
        <button
          on:click={handleConfirm}
          disabled={!newName.trim() || newName === currentName}
          class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 
                 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Rename
        </button>
      </div>
    </div>
  </div>
{/if}
