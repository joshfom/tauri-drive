<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  // Props
  export let items: any[] = [];
  export let itemHeight: number = 60;
  export let overscan: number = 5;

  // Internal state
  let containerEl: HTMLDivElement;
  let scrollTop = 0;
  let containerHeight = 0;

  // Calculate visible range
  $: totalHeight = items.length * itemHeight;
  $: startIndex = Math.max(0, Math.floor(scrollTop / itemHeight) - overscan);
  $: endIndex = Math.min(items.length, Math.ceil((scrollTop + containerHeight) / itemHeight) + overscan);
  $: visibleItems = items.slice(startIndex, endIndex).map((item, i) => ({
    item,
    index: startIndex + i,
    style: `position: absolute; top: ${(startIndex + i) * itemHeight}px; width: 100%; height: ${itemHeight}px;`
  }));

  function handleScroll(e: Event) {
    const target = e.target as HTMLDivElement;
    scrollTop = target.scrollTop;
  }

  let resizeObserver: ResizeObserver;

  onMount(() => {
    if (containerEl) {
      containerHeight = containerEl.clientHeight;
      resizeObserver = new ResizeObserver((entries) => {
        for (const entry of entries) {
          containerHeight = entry.contentRect.height;
        }
      });
      resizeObserver.observe(containerEl);
    }
  });

  onDestroy(() => {
    if (resizeObserver) {
      resizeObserver.disconnect();
    }
  });
</script>

<div 
  bind:this={containerEl}
  class="virtual-list-container"
  on:scroll={handleScroll}
>
  <div class="virtual-list-inner" style="height: {totalHeight}px; position: relative;">
    {#each visibleItems as { item, index, style } (index)}
      <div {style}>
        <slot {item} {index} />
      </div>
    {/each}
  </div>
</div>

<style>
  .virtual-list-container {
    overflow-y: auto;
    height: 100%;
    width: 100%;
  }
</style>
