<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
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

  function handleEdit() {
    showCredentials = true;
    isConnected = false;
  }
</script>

<div class="flex flex-col h-full bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900 dark:to-gray-800">
  <!-- Header -->
  <div class="bg-white/80 dark:bg-gray-800/80 backdrop-blur-sm border-b border-gray-200/50 dark:border-gray-700/50 px-8 py-6 shadow-sm">
    <div>
      <h2 class="text-3xl font-bold text-gray-900 dark:text-white">Settings</h2>
      <p class="text-sm text-gray-500 dark:text-gray-400 mt-1">Configure your Cloudflare R2 credentials</p>
    </div>
  </div>

  <!-- Content -->
  <div class="flex-1 overflow-auto p-8">
    <div class="max-w-2xl mx-auto">
      <div class="bg-white/80 dark:bg-gray-800/80 backdrop-blur-sm shadow-xl rounded-2xl p-8 border border-gray-200/50 dark:border-gray-700/50">
        <div class="flex items-center space-x-3 mb-6">
          <div class="w-12 h-12 bg-gradient-to-br from-blue-500 to-blue-600 rounded-xl flex items-center justify-center shadow-lg">
            <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z"/>
            </svg>
          </div>
          <div>
            <h3 class="text-xl font-bold text-gray-900 dark:text-white">
              Cloudflare R2 Configuration
            </h3>
            <p class="text-sm text-gray-500 dark:text-gray-400">Connect to your R2 storage bucket</p>
          </div>
        </div>

        {#if message}
          <div
            class="mb-6 p-4 rounded-xl border-l-4 {messageType === 'success'
              ? 'bg-gradient-to-r from-green-50 to-green-100 dark:from-green-900/20 dark:to-green-800/20 border-green-500'
              : 'bg-gradient-to-r from-red-50 to-red-100 dark:from-red-900/20 dark:to-red-800/20 border-red-500'}"
          >
            <div class="flex items-start">
              {#if messageType === 'success'}
                <svg class="w-5 h-5 text-green-600 dark:text-green-400 mt-0.5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
              {:else}
                <svg class="w-5 h-5 text-red-600 dark:text-red-400 mt-0.5 mr-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
              {/if}
              <p
                class="{messageType === 'success'
                  ? 'text-green-800 dark:text-green-200'
                  : 'text-red-800 dark:text-red-200'}"
              >
                {message}
              </p>
            </div>
          </div>
        {/if}

        {#if isConnected && !showCredentials}
          <!-- Connected State - Show Bucket Info -->
          <div class="space-y-6">
            <div class="bg-gradient-to-r from-green-50 to-green-100 dark:from-green-900/20 dark:to-green-800/20 rounded-xl p-6 border border-green-200 dark:border-green-800">
              <div class="flex items-center justify-between">
                <div class="flex items-center space-x-4">
                  <div class="w-12 h-12 bg-green-500 rounded-xl flex items-center justify-center">
                    <svg class="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                    </svg>
                  </div>
                  <div>
                    <p class="text-sm font-medium text-green-700 dark:text-green-300">Connected to</p>
                    <p class="text-2xl font-bold text-green-900 dark:text-green-100">{currentBucket}</p>
                  </div>
                </div>
                <button
                  on:click={handleEdit}
                  class="px-4 py-2 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 border border-gray-300 dark:border-gray-600 rounded-xl hover:bg-gray-50 dark:hover:bg-gray-600 transition-all duration-200 flex items-center space-x-2"
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.232 5.232l3.536 3.536m-2.036-5.036a2.5 2.5 0 113.536 3.536L6.5 21.036H3v-3.572L16.732 3.732z"/>
                  </svg>
                  <span>Edit</span>
                </button>
              </div>
            </div>

            <div class="bg-blue-50 dark:bg-blue-900/20 rounded-xl p-4 border border-blue-200 dark:border-blue-800">
              <div class="flex items-start space-x-3">
                <svg class="w-5 h-5 text-blue-600 dark:text-blue-400 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                <div class="flex-1">
                  <p class="text-sm font-semibold text-blue-900 dark:text-blue-100">Credentials Saved</p>
                  <p class="text-xs text-blue-700 dark:text-blue-300 mt-1">Your credentials are securely stored and will be used automatically when you restart the app.</p>
                </div>
              </div>
            </div>
          </div>
        {:else}
          <!-- Credentials Form -->
          <div class="space-y-5">
          <div>
            <label
              for="accountId"
              class="block text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2"
            >
              Account ID
            </label>
            <input
              id="accountId"
              type="text"
              bind:value={accountId}
              class="w-full px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-700/50 dark:text-white transition-all duration-200"
              placeholder="your-account-id"
            />
          </div>

          <div>
            <label
              for="accessKeyId"
              class="block text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2"
            >
              Access Key ID
            </label>
            <input
              id="accessKeyId"
              type="text"
              bind:value={accessKeyId}
              class="w-full px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-700/50 dark:text-white transition-all duration-200"
              placeholder="Your access key"
            />
          </div>

          <div>
            <label
              for="secretAccessKey"
              class="block text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2"
            >
              Secret Access Key
            </label>
            <input
              id="secretAccessKey"
              type="password"
              bind:value={secretAccessKey}
              class="w-full px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-700/50 dark:text-white transition-all duration-200"
              placeholder="Your secret key"
            />
          </div>

          <div>
            <label
              for="bucketName"
              class="block text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2"
            >
              Bucket Name
            </label>
            <input
              id="bucketName"
              type="text"
              bind:value={bucketName}
              class="w-full px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-xl focus:ring-2 focus:ring-blue-500 focus:border-transparent dark:bg-gray-700/50 dark:text-white transition-all duration-200"
              placeholder="my-bucket"
            />
          </div>

          <button
            on:click={handleConnect}
            disabled={connecting}
            class="w-full px-6 py-3.5 bg-gradient-to-r from-blue-600 to-blue-700 text-white rounded-xl hover:from-blue-700 hover:to-blue-800 disabled:from-gray-400 disabled:to-gray-500 disabled:cursor-not-allowed transition-all duration-200 shadow-lg hover:shadow-xl transform hover:-translate-y-0.5 flex items-center justify-center space-x-2 font-semibold"
          >
            {#if connecting}
              <svg class="animate-spin h-5 w-5" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              <span>Connecting...</span>
            {:else}
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
              </svg>
              <span>Connect to R2</span>
            {/if}
          </button>
          </div>

          <div class="mt-6 pt-6 border-t border-gray-200 dark:border-gray-700">
            <p class="text-sm text-gray-500 dark:text-gray-400">
              <strong>How to get your R2 credentials:</strong><br />
              1. Go to Cloudflare Dashboard → R2<br />
              2. Create or select a bucket<br />
              3. Go to "Manage R2 API Tokens"<br />
              4. Create a new API token with read/write permissions
            </p>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>
