<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { save, open } from '@tauri-apps/plugin-dialog';
  import { check } from '@tauri-apps/plugin-updater';
  import { relaunch } from '@tauri-apps/plugin-process';
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
  
  // Update state
  let checkingForUpdates = false;
  let updateCheckResult: 'none' | 'available' | 'latest' | 'error' = 'none';
  let updateVersion = '';
  let updateError = '';

  // Migration state
  let showMigrationExport = false;
  let showMigrationImport = false;
  let migrationPassword = '';
  let migrationConfirmPassword = '';
  let migrationExporting = false;
  let migrationImporting = false;
  let migrationPreview: MigrationPreview | null = null;
  let migrationError = '';

  interface MigrationPreview {
    version: number;
    app_version: string;
    created_at: string;
    has_credentials: boolean;
    bucket_name: string | null;
    sync_folders_count: number;
    settings_count: number;
    upload_history_count: number;
  }

  interface MigrationResult {
    credentials_imported: boolean;
    sync_folders_imported: number;
    settings_imported: number;
    upload_history_imported: number;
  }

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

  async function checkForUpdates() {
    checkingForUpdates = true;
    updateError = '';
    updateCheckResult = 'none';
    
    try {
      const update = await check();
      
      if (update) {
        updateCheckResult = 'available';
        updateVersion = update.version;
        // Store the update object globally so UpdateScreen in App.svelte can use it
        (window as any).__pendingUpdate = update;
        // Dispatch custom event to show update screen
        window.dispatchEvent(new CustomEvent('show-update-screen', { detail: update }));
      } else {
        updateCheckResult = 'latest';
        message = 'You are running the latest version!';
        messageType = 'success';
      }
    } catch (e) {
      updateCheckResult = 'error';
      updateError = `Failed to check for updates: ${e}`;
    } finally {
      checkingForUpdates = false;
    }
  }

  async function handleMigrationExport() {
    if (migrationPassword.length < 6) {
      migrationError = 'Password must be at least 6 characters';
      return;
    }
    if (migrationPassword !== migrationConfirmPassword) {
      migrationError = 'Passwords do not match';
      return;
    }

    migrationError = '';
    migrationExporting = true;

    try {
      const savePath = await save({
        defaultPath: 'cloudflare-backup-migration.tdbak',
        filters: [{ name: 'Tauri Drive Backup', extensions: ['tdbak'] }],
      });

      if (!savePath) {
        migrationExporting = false;
        return;
      }

      await invoke('export_migration_backup', { 
        filePath: savePath, 
        password: migrationPassword 
      });
      
      message = 'Migration backup exported successfully! Keep the password safe.';
      messageType = 'success';
      showMigrationExport = false;
      migrationPassword = '';
      migrationConfirmPassword = '';
    } catch (e) {
      migrationError = `Export failed: ${e}`;
    } finally {
      migrationExporting = false;
    }
  }

  async function handleMigrationImportSelect() {
    migrationError = '';
    migrationPreview = null;

    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: 'Tauri Drive Backup', extensions: ['tdbak'] }],
      });

      if (!selected || Array.isArray(selected)) {
        return;
      }

      // Store the selected file path
      (window as any).__migrationFilePath = selected;
      showMigrationImport = true;
    } catch (e) {
      migrationError = `Failed to select file: ${e}`;
    }
  }

  async function handleMigrationPreview() {
    if (migrationPassword.length < 6) {
      migrationError = 'Please enter your backup password';
      return;
    }

    migrationError = '';
    migrationImporting = true;

    try {
      const filePath = (window as any).__migrationFilePath;
      migrationPreview = await invoke<MigrationPreview>('preview_migration_backup', {
        filePath,
        password: migrationPassword,
      });
    } catch (e) {
      migrationError = `${e}`;
      migrationPreview = null;
    } finally {
      migrationImporting = false;
    }
  }

  async function handleMigrationImport() {
    migrationError = '';
    migrationImporting = true;

    try {
      const filePath = (window as any).__migrationFilePath;
      const result = await invoke<MigrationResult>('import_migration_backup', {
        filePath,
        password: migrationPassword,
      });

      let summary = 'Migration complete! Imported: ';
      const parts = [];
      if (result.credentials_imported) parts.push('R2 credentials');
      if (result.sync_folders_imported > 0) parts.push(`${result.sync_folders_imported} sync folders`);
      if (result.settings_imported > 0) parts.push(`${result.settings_imported} settings`);
      
      message = summary + (parts.length > 0 ? parts.join(', ') : 'nothing new');
      messageType = 'success';
      
      showMigrationImport = false;
      migrationPassword = '';
      migrationPreview = null;

      // Reload to apply imported settings
      if (result.credentials_imported) {
        try {
          const loadResult = await invoke<string>('load_and_connect');
          const savedBucket = await invoke<string | null>('get_saved_bucket');
          if (savedBucket) {
            currentBucket = savedBucket;
            isConnected = true;
            showCredentials = false;
          }
        } catch (e) {
          // Connection may fail, that's okay
        }
      }
    } catch (e) {
      migrationError = `Import failed: ${e}`;
    } finally {
      migrationImporting = false;
    }
  }

  function closeMigrationDialogs() {
    showMigrationExport = false;
    showMigrationImport = false;
    migrationPassword = '';
    migrationConfirmPassword = '';
    migrationPreview = null;
    migrationError = '';
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

      <!-- Move to New Computer -->
      <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl">
        <div class="p-4 border-b border-gray-200 dark:border-gray-700">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 bg-purple-500 rounded-lg flex items-center justify-center">
              <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4"/>
              </svg>
            </div>
            <div>
              <h3 class="font-medium text-gray-900 dark:text-white">Move to New Computer</h3>
              <p class="text-sm text-gray-500 dark:text-gray-400">Transfer all settings and data securely</p>
            </div>
          </div>
        </div>
        
        <div class="p-4 space-y-4">
          <p class="text-sm text-gray-600 dark:text-gray-400">
            Create an encrypted backup of your entire setup including credentials, sync folders, and settings. 
            Import on your new computer to restore everything.
          </p>
          
          <div class="flex gap-3">
            <button
              on:click={() => { showMigrationExport = true; migrationError = ''; }}
              disabled={!isConnected}
              class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium text-white bg-purple-600 hover:bg-purple-700 disabled:bg-gray-400 rounded-lg transition-colors"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
              </svg>
              Create Backup
            </button>
            
            <button
              on:click={handleMigrationImportSelect}
              class="flex-1 flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium text-purple-700 dark:text-purple-300 border border-purple-300 dark:border-purple-600 rounded-lg hover:bg-purple-50 dark:hover:bg-purple-900/20 transition-colors"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 4H6a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2V6a2 2 0 00-2-2h-2m-4-1v8m0 0l3-3m-3 3L9 8m-5 5h2.586a1 1 0 01.707.293l2.414 2.414a1 1 0 00.707.293h3.172a1 1 0 00.707-.293l2.414-2.414a1 1 0 01.707-.293H20"/>
              </svg>
              Restore Backup
            </button>
          </div>
          
          <p class="text-xs text-gray-500 dark:text-gray-400">
            Backups are encrypted with a password you choose. Keep your password safe!
          </p>
        </div>
      </div>

      <!-- About & Updates -->
      <div class="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl">
        <div class="p-4 border-b border-gray-200 dark:border-gray-700">
          <h3 class="font-medium text-gray-900 dark:text-white">About & Updates</h3>
          <p class="text-sm text-gray-500 dark:text-gray-400">
            A secure desktop application for backing up files to Cloudflare R2 storage.
          </p>
          <p class="text-xs text-gray-400 dark:text-gray-500 mt-2">Version 1.0.0</p>
        </div>
        
        <div class="p-4 space-y-4">
          <!-- Update Error -->
          {#if updateError}
            <div class="p-3 bg-red-50 dark:bg-red-900/10 border border-red-200 dark:border-red-800 rounded-lg">
              <p class="text-sm text-red-800 dark:text-red-200">{updateError}</p>
            </div>
          {/if}

          <!-- Update Available Notice -->
          {#if updateCheckResult === 'available'}
            <div class="p-3 bg-blue-50 dark:bg-blue-900/10 border border-blue-200 dark:border-blue-800 rounded-lg">
              <div class="flex items-center gap-2">
                <svg class="w-5 h-5 text-blue-600 dark:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10"/>
                </svg>
                <span class="text-sm font-medium text-blue-800 dark:text-blue-200">Update v{updateVersion} available!</span>
              </div>
            </div>
          {/if}

          <!-- Check for Updates Button -->
          <button
            on:click={checkForUpdates}
            disabled={checkingForUpdates}
            class="w-full flex items-center justify-center gap-2 px-4 py-2.5 text-sm font-medium text-gray-700 dark:text-gray-300 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 transition-colors"
          >
            {#if checkingForUpdates}
              <svg class="animate-spin h-4 w-4" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span>Checking for updates...</span>
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
              </svg>
              <span>Check for Updates</span>
            {/if}
          </button>

          <p class="text-xs text-gray-500 dark:text-gray-400 text-center">
            Updates are checked automatically when the app starts.
          </p>
        </div>
      </div>
    </div>
  </div>
</div>

<!-- Migration Export Modal -->
{#if showMigrationExport}
  <div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-2xl shadow-xl max-w-md w-full overflow-hidden">
      <div class="p-6 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 bg-purple-500 rounded-lg flex items-center justify-center">
            <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
            </svg>
          </div>
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Create Encrypted Backup</h3>
            <p class="text-sm text-gray-500 dark:text-gray-400">Secure your data with a password</p>
          </div>
        </div>
      </div>
      
      <div class="p-6 space-y-4">
        {#if migrationError}
          <div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
            <p class="text-sm text-red-700 dark:text-red-400">{migrationError}</p>
          </div>
        {/if}
        
        <div class="space-y-2">
          <label for="migration-password" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Password (minimum 6 characters)
          </label>
          <input
            id="migration-password"
            type="password"
            bind:value={migrationPassword}
            placeholder="Enter a strong password"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-purple-500 focus:border-transparent"
          />
        </div>
        
        <div class="space-y-2">
          <label for="migration-confirm-password" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
            Confirm Password
          </label>
          <input
            id="migration-confirm-password"
            type="password"
            bind:value={migrationConfirmPassword}
            placeholder="Confirm your password"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-purple-500 focus:border-transparent"
          />
        </div>
        
        <div class="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
          <p class="text-sm text-amber-700 dark:text-amber-400">
            ⚠️ Keep this password safe! You'll need it to restore the backup on your new computer.
          </p>
        </div>
      </div>
      
      <div class="p-6 border-t border-gray-200 dark:border-gray-700 flex gap-3 justify-end">
        <button
          on:click={closeMigrationDialogs}
          disabled={migrationExporting}
          class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors disabled:opacity-50"
        >
          Cancel
        </button>
        <button
          on:click={handleMigrationExport}
          disabled={migrationExporting || migrationPassword.length < 6}
          class="px-4 py-2 text-sm font-medium text-white bg-purple-600 hover:bg-purple-700 disabled:bg-gray-400 rounded-lg transition-colors flex items-center gap-2"
        >
          {#if migrationExporting}
            <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
            </svg>
            Creating Backup...
          {:else}
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"/>
            </svg>
            Create & Save Backup
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Migration Import Modal -->
{#if showMigrationImport}
  <div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50 p-4">
    <div class="bg-white dark:bg-gray-800 rounded-2xl shadow-xl max-w-md w-full overflow-hidden">
      <div class="p-6 border-b border-gray-200 dark:border-gray-700">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 bg-purple-500 rounded-lg flex items-center justify-center">
            <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 4H6a2 2 0 00-2 2v12a2 2 0 002 2h12a2 2 0 002-2V6a2 2 0 00-2-2h-2m-4-1v8m0 0l3-3m-3 3L9 8m-5 5h2.586a1 1 0 01.707.293l2.414 2.414a1 1 0 00.707.293h3.172a1 1 0 00.707-.293l2.414-2.414a1 1 0 01.707-.293H20"/>
            </svg>
          </div>
          <div>
            <h3 class="text-lg font-semibold text-gray-900 dark:text-white">Restore Backup</h3>
            <p class="text-sm text-gray-500 dark:text-gray-400">Import settings from another device</p>
          </div>
        </div>
      </div>
      
      <div class="p-6 space-y-4">
        {#if migrationError}
          <div class="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
            <p class="text-sm text-red-700 dark:text-red-400">{migrationError}</p>
          </div>
        {/if}
        
        {#if !migrationPreview}
          <!-- Password entry step -->
          <div class="space-y-2">
            <label for="migration-import-password" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Enter Backup Password
            </label>
            <input
              id="migration-import-password"
              type="password"
              bind:value={migrationPassword}
              placeholder="Password used when creating the backup"
              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-purple-500 focus:border-transparent"
            />
          </div>
          
          <div class="flex gap-3 justify-end">
            <button
              on:click={closeMigrationDialogs}
              class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
            >
              Cancel
            </button>
            <button
              on:click={handleMigrationPreview}
              disabled={migrationPassword.length < 6}
              class="px-4 py-2 text-sm font-medium text-white bg-purple-600 hover:bg-purple-700 disabled:bg-gray-400 rounded-lg transition-colors"
            >
              Decrypt & Preview
            </button>
          </div>
        {:else}
          <!-- Preview step -->
          <div class="space-y-3">
            <div class="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">Backup Created</p>
              <p class="text-sm font-medium text-gray-900 dark:text-white">
                {new Date(migrationPreview.created_at).toLocaleString()}
              </p>
            </div>
            
            <div class="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
              <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">App Version</p>
              <p class="text-sm font-medium text-gray-900 dark:text-white">
                {migrationPreview.app_version}
              </p>
            </div>
            
            <div class="grid grid-cols-2 gap-3">
              <div class="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-center">
                <p class="text-2xl font-bold text-purple-600">{migrationPreview.has_credentials ? 1 : 0}</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">Credentials</p>
                {#if migrationPreview.bucket_name}
                  <p class="text-xs text-gray-400 truncate">{migrationPreview.bucket_name}</p>
                {/if}
              </div>
              <div class="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-center">
                <p class="text-2xl font-bold text-purple-600">{migrationPreview.sync_folders_count}</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">Sync Folders</p>
              </div>
              <div class="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-center">
                <p class="text-2xl font-bold text-purple-600">{migrationPreview.settings_count}</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">Settings</p>
              </div>
              <div class="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-center">
                <p class="text-2xl font-bold text-purple-600">{migrationPreview.upload_history_count}</p>
                <p class="text-xs text-gray-500 dark:text-gray-400">Upload Records</p>
              </div>
            </div>
            
            <div class="p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg">
              <p class="text-sm text-amber-700 dark:text-amber-400">
                ⚠️ This will replace your current settings. Make sure you want to proceed.
              </p>
            </div>
          </div>
          
          <div class="flex gap-3 justify-end pt-2">
            <button
              on:click={closeMigrationDialogs}
              disabled={migrationImporting}
              class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              on:click={handleMigrationImport}
              disabled={migrationImporting}
              class="px-4 py-2 text-sm font-medium text-white bg-purple-600 hover:bg-purple-700 disabled:bg-gray-400 rounded-lg transition-colors flex items-center gap-2"
            >
              {#if migrationImporting}
                <svg class="w-4 h-4 animate-spin" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                Importing...
              {:else}
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                </svg>
                Import & Restore
              {/if}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
