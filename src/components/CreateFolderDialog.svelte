<script lang="ts">
  export let isOpen = false;
  export let currentPath = '';
  export let onConfirm: (folderName: string) => void;
  export let onCancel: () => void;

  let folderName = '';
  let error = '';

  function handleConfirm() {
    // Validate folder name
    if (!folderName.trim()) {
      error = 'Folder name cannot be empty';
      return;
    }

    if (folderName.includes('/') || folderName.includes('\\')) {
      error = 'Folder name cannot contain slashes';
      return;
    }

    if (folderName.startsWith('.')) {
      error = 'Folder name cannot start with a dot';
      return;
    }

    onConfirm(folderName.trim());
    isOpen = false;
    folderName = '';
    error = '';
  }

  function handleCancel() {
    onCancel();
    isOpen = false;
    folderName = '';
    error = '';
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleCancel();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      handleCancel();
    } else if (e.key === 'Enter') {
      handleConfirm();
    }
  }
</script>

{#if isOpen}
  <!-- Backdrop -->
  <div 
    class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4"
    on:click={handleBackdropClick}
    on:keydown={handleKeydown}
    role="button"
    tabindex="-1"
  >
    <!-- Dialog -->
    <div class="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl max-w-md w-full p-6 transform transition-all">
      <!-- Icon -->
      <div class="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-blue-100 dark:bg-blue-900/30 mb-4">
        <svg class="h-6 w-6 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z"/>
        </svg>
      </div>

      <!-- Title -->
      <h3 class="text-lg font-bold text-gray-900 dark:text-white text-center mb-2">
        Create New Folder
      </h3>

      <!-- Current path -->
      {#if currentPath}
        <p class="text-xs text-gray-500 dark:text-gray-400 text-center mb-4">
          in /{currentPath}
        </p>
      {:else}
        <p class="text-xs text-gray-500 dark:text-gray-400 text-center mb-4">
          in root directory
        </p>
      {/if}

      <!-- Input -->
      <div class="mb-4">
        <label for="folderName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          Folder Name
        </label>
        <input
          id="folderName"
          type="text"
          bind:value={folderName}
          placeholder="Enter folder name"
          class="w-full px-4 py-2.5 bg-gray-50 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent text-gray-900 dark:text-white placeholder-gray-400"
        />
        {#if error}
          <p class="mt-2 text-sm text-red-600 dark:text-red-400">
            {error}
          </p>
        {/if}
      </div>

      <!-- Actions -->
      <div class="flex space-x-3">
        <button
          on:click={handleCancel}
          class="flex-1 px-4 py-2.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200 rounded-xl hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors duration-200 font-medium"
        >
          Cancel
        </button>
        <button
          on:click={handleConfirm}
          class="flex-1 px-4 py-2.5 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-700 hover:to-blue-800 text-white rounded-xl transition-all duration-200 shadow-lg hover:shadow-xl font-medium"
        >
          Create
        </button>
      </div>
    </div>
  </div>
{/if}
