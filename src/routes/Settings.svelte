<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save, open } from '@tauri-apps/plugin-dialog';
  import type { R2Credentials } from '../lib/types';

  let accountId = '';
  let accessKeyId = '';
  let secretAccessKey = '';
  let bucketName = '';
  let connecting = false;
  let message = '';
  let messageType: 'success' | 'error' = 'success';
  let isConnected = false;
  let currentBucket = '';
  let showCredentials = false;
  let exporting = false;
  let importing = false;

  onMount(async () => {
    // Try to load saved bucket
    try {
      const savedBucket = await invoke<string | null>('get_saved_bucket');
      if (savedBucket) {
        currentBucket = savedBucket;
        // Try to auto-connect
        try {
          const result = await invoke<string>('load_and_connect');
          isConnected = true;
          message = result;
          messageType = 'success';
        } catch (e) {
          // If auto-connect fails, show credentials form
          showCredentials = true;
        }
      } else {
        showCredentials = true;
      }
    } catch (e) {
      showCredentials = true;
    }
  });

  async function handleConnect() {
    if (!accountId || !accessKeyId || !secretAccessKey || !bucketName) {
      message = 'Please fill in all fields';
      messageType = 'error';
      return;
    }

    connecting = true;
    message = '';

    try {
      const credentials: R2Credentials = {
        accountId,
        accessKeyId,
        secretAccessKey,
        endpoint: `https://${accountId}.r2.cloudflarestorage.com`,
      };

      const result = await invoke<string>('connect_r2', {
        credentials,
        bucket: bucketName,
        saveCredentials: true,
      });

      message = result;
      messageType = 'success';
      isConnected = true;
      currentBucket = bucketName;
      showCredentials = false;
      
      // Clear sensitive data
      accountId = '';
      accessKeyId = '';
      secretAccessKey = '';
    } catch (e) {
      message = `Connection failed: ${e}`;
      messageType = 'error';
    } finally {
      connecting = false;
    }
  }

  async function handleEdit() {
    // Load current credentials for editing
    try {
      const creds = await invoke<[string, string, string, string] | null>('get_current_credentials');
      if (creds) {
        bucketName = creds[0];
        accountId = creds[1];
        accessKeyId = creds[2];
        secretAccessKey = creds[3];
      }
    } catch (e) {
      console.error('Failed to load credentials:', e);
    }
    showCredentials = true;
    isConnected = false;
  }

  async function handleExportConfig() {
    try {
      exporting = true;
      const savePath = await save({
        defaultPath: 'cloudflare-backup-config.json',
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });

      if (!savePath) {
        exporting = false;
        return;
      }

      await invoke('export_config', { filePath: savePath });
      message = 'Configuration exported successfully';
      messageType = 'success';
    } catch (e) {
      message = `Export failed: ${e}`;
      messageType = 'error';
    } finally {
      exporting = false;
    }
  }

  async function handleImportConfig() {
    try {
      importing = true;
      const selected = await open({
        multiple: false,
        filters: [{ name: 'JSON', extensions: ['json'] }],
      });

      if (!selected || Array.isArray(selected)) {
        importing = false;
        return;
      }

      await invoke('import_config', { filePath: selected });
      message = 'Configuration imported successfully. Reconnecting...';
      messageType = 'success';
      
      // Try to reconnect with imported config
      try {
        const result = await invoke<string>('load_and_connect');
        const savedBucket = await invoke<string | null>('get_saved_bucket');
        if (savedBucket) {
          currentBucket = savedBucket;
          isConnected = true;
          showCredentials = false;
        }
        message = 'Configuration imported and connected successfully';
      } catch (e) {
        showCredentials = true;
        message = 'Configuration imported but connection failed. Please verify credentials.';
        messageType = 'error';
      }
    } catch (e) {
      message = `Import failed: ${e}`;
      messageType = 'error';
    } finally {
      importing = false;
    }
  }
</script>

<div class="flex flex-col h-full bg-white dark:bg-gray-900">
  <!-- Header - Flat design -->
  <div class="border-b border-gray-200 dark:border-gray-800 px-6 py-4">
    <h2 class="text-xl font-semibold text-gray-900 dark:text-white">Settings</h2>
    <p class="text-sm text-gray-500 dark:text-gray-400 mt-0.5">Manage your Cloudflare R2 connection</p>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-auto p-6">
    <div class="max-w-xl mx-auto space-y-6">
      <!-- Message -->
      {#if message}
        <div class="p-3 rounded-lg {messageType === 'success' ? 'bg-green-50 dark:bg-green-900/10 border border-green-200 dark:border-green-800' : 'bg-red-50 dark:bg-red-900/10 border border-red-200 dark:border-red-800'}">
          <div class="flex items-center gap-2">
            {#if messageType === 'success'}
              <svg class="w-4 h-4 text-green-600 dark:text-green-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
              </svg>
            {:else}
              <svg class="w-4 h-4 text-red-600 dark:text-red-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
              </svg>
            {/if}
            <p class="text-sm {messageType === 'success' ? 'text-green-800 dark:text-green-200' : 'text-red-800 dark:text-red-200'}">{message}</p>
          </div>
        </div>
      {/if}

      <!-- Connection Card -->
      <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl">
        <div class="p-4 border-b border-gray-200 dark:border-gray-700">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 bg-orange-500 rounded-lg flex items-center justify-center">
              <svg class="w-5 h-5 text-white" viewBox="0 0 24 24" fill="currentColor">
                <path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>
              </svg>
            </div>
            <div>
              <h3 class="font-medium text-gray-900 dark:text-white">R2 Connection</h3>
              <p class="text-sm text-gray-500 dark:text-gray-400">Connect to your Cloudflare R2 bucket</p>
            </div>
          </div>
        </div>

        {#if isConnected && !showCredentials}
          <!-- Connected State -->
          <div class="p-4 space-y-4">
            <div class="flex items-center justify-between p-3 bg-green-50 dark:bg-green-900/10 border border-green-200 dark:border-green-800 rounded-lg">
              <div class="flex items-center gap-3">
                <div class="w-2 h-2 rounded-full bg-green-500"></div>
                <div>
                  <p class="text-sm font-medium text-green-800 dark:text-green-200">Connected</p>
                  <p class="text-xs text-green-600 dark:text-green-400">{currentBucket}</p>
                </div>
              </div>
              <button
                on:click={handleEdit}
                class="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
              >
                Change
              </button>
            </div>

            <p class="text-xs text-gray-500 dark:text-gray-400">
              Your credentials are encrypted and stored securely on your device.
            </p>
          </div>
        {:else}
          <!-- Credentials Form -->
          <div class="p-4 space-y-4">
            <div>
              <label for="accountId" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                Account ID
              </label>
              <input
                id="accountId"
                type="text"
                bind:value={accountId}
                class="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-900 dark:text-white"
                placeholder="your-account-id"
              />
            </div>

            <div>
              <label for="accessKeyId" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                Access Key ID
              </label>
              <input
                id="accessKeyId"
                type="text"
                bind:value={accessKeyId}
                class="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-900 dark:text-white"
                placeholder="Your access key"
              />
            </div>

            <div>
              <label for="secretAccessKey" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                Secret Access Key
              </label>
              <input
                id="secretAccessKey"
                type="password"
                bind:value={secretAccessKey}
                class="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-900 dark:text-white"
                placeholder="Your secret key"
              />
            </div>

            <div>
              <label for="bucketName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                Bucket Name
              </label>
              <input
                id="bucketName"
                type="text"
                bind:value={bucketName}
                class="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-900 dark:text-white"
                placeholder="my-bucket"
              />
            </div>

            <button
              on:click={handleConnect}
              disabled={connecting}
              class="w-full px-4 py-2.5 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 disabled:bg-gray-400 rounded-lg transition-colors flex items-center justify-center gap-2"
            >
              {#if connecting}
                <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                <span>Connecting...</span>
              {:else}
                <span>Connect</span>
              {/if}
            </button>

            <div class="pt-3 border-t border-gray-200 dark:border-gray-700">
              <p class="text-xs text-gray-500 dark:text-gray-400">
                Get your R2 credentials from Cloudflare Dashboard → R2 → Manage R2 API Tokens
              </p>
            </div>
          </div>
        {/if}
      </div>

      <!-- Export/Import Configuration -->
      <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl">
        <div class="p-4 border-b border-gray-200 dark:border-gray-700">
          <h3 class="font-medium text-gray-900 dark:text-white">Configuration</h3>
          <p class="text-sm text-gray-500 dark:text-gray-400">Export or import your settings</p>
        </div>
        
        <div class="p-4 space-y-3">
          <div class="flex gap-3">
            <button
              on:click={handleExportConfig}
              disabled={exporting || !isConnected}
              class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 transition-colors"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
              </svg>
              {exporting ? 'Exporting...' : 'Export'}
            </button>
            
            <button
              on:click={handleImportConfig}
              disabled={importing}
              class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 transition-colors"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4"/>
              </svg>
              {importing ? 'Importing...' : 'Import'}
            </button>
          </div>
          
          <p class="text-xs text-gray-500 dark:text-gray-400">
            Export your configuration to transfer settings to another device. Credentials are encrypted in the export file.
          </p>
        </div>
      </div>

      <!-- About -->
      <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl">
        <div class="p-4">
          <h3 class="font-medium text-gray-900 dark:text-white mb-1">Cloudflare Backup</h3>
          <p class="text-sm text-gray-500 dark:text-gray-400">
            A secure desktop application for backing up files to Cloudflare R2 storage.
          </p>
          <p class="text-xs text-gray-400 dark:text-gray-500 mt-2">Version 1.0.0</p>
        </div>
      </div>
    </div>
  </div>
</div>
