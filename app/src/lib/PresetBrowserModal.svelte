<script lang="ts">
  import type { ChannelId, Preset } from '../types';
  import { X, Search, Check, Sliders } from '@lucide/svelte';

  export let isOpen: boolean;
  export let activeChannel: ChannelId = 'game';
  export let allPresets: Preset[];
  export let activePresetId: string;
  export let onClose: () => void;
  export let onSelectPreset: (preset: Preset) => void;

  let selectedCategory: 'all' | 'game' | 'chat' | 'media' | 'mic' = 'all';
  let searchQuery = '';

  $: if (isOpen && activeChannel) {
    if (activeChannel === 'game' || activeChannel === 'chat' || activeChannel === 'media' || activeChannel === 'mic') {
      selectedCategory = activeChannel;
    } else {
      selectedCategory = 'all';
    }
  }

  $: filteredPresets = allPresets.filter(p => {
    const matchCat = selectedCategory === 'all' || p.category === selectedCategory;
    const matchSearch =
      searchQuery.trim() === '' ||
      p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
      p.tags.some(t => t.toLowerCase().includes(searchQuery.toLowerCase()));
    return matchCat && matchSearch;
  });
</script>

{#if isOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    on:click={onClose}
    on:keydown={(e) => e.key === 'Escape' && onClose()}
  >
    <div
      class="modal-window"
      role="dialog"
      aria-modal="true"
      aria-label="Preset Browser Modal"
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <div class="modal-header">
        <div class="header-left">
          <Sliders size={18} color="var(--color-accent-brand)" />
          <span class="modal-title">BROWSE EQUALIZER PRESETS</span>
        </div>
        <button class="close-btn" aria-label="Close Preset Browser" on:click={onClose}>
          <X size={18} />
        </button>
      </div>

      <div class="modal-toolbar">
        <div class="category-tabs" role="tablist" aria-label="Preset Categories">
          <button
            class="cat-btn"
            class:active={selectedCategory === 'all'}
            aria-current={selectedCategory === 'all' ? 'true' : undefined}
            on:click={() => selectedCategory = 'all'}
          >ALL</button>
          <button
            class="cat-btn"
            class:active={selectedCategory === 'game'}
            aria-current={selectedCategory === 'game' ? 'true' : undefined}
            on:click={() => selectedCategory = 'game'}
          >GAME</button>
          <button
            class="cat-btn"
            class:active={selectedCategory === 'chat'}
            aria-current={selectedCategory === 'chat' ? 'true' : undefined}
            on:click={() => selectedCategory = 'chat'}
          >CHAT</button>
          <button
            class="cat-btn"
            class:active={selectedCategory === 'media'}
            aria-current={selectedCategory === 'media' ? 'true' : undefined}
            on:click={() => selectedCategory = 'media'}
          >MEDIA</button>
          <button
            class="cat-btn"
            class:active={selectedCategory === 'mic'}
            aria-current={selectedCategory === 'mic' ? 'true' : undefined}
            on:click={() => selectedCategory = 'mic'}
          >MIC</button>
        </div>

        <div class="search-box">
          <Search size={14} color="var(--color-text-muted)" />
          <input
            type="text"
            aria-label="Search Presets"
            placeholder="Search game (CS2, Apex...), tags, or profile..."
            bind:value={searchQuery}
          />
        </div>
      </div>

      <div class="preset-grid">
        {#each filteredPresets as preset}
          {@const isActive = preset.id === activePresetId}
          <button
            type="button"
            class="preset-card"
            class:active={isActive}
            aria-label="Preset: {preset.name}, {preset.description}"
            on:click={() => onSelectPreset(preset)}
          >
            <div class="card-top">
              <span class="category-badge">{preset.category.toUpperCase()}</span>
              {#if isActive}
                <div class="active-badge">
                  <Check size={12} color="var(--color-accent-brand)" />
                  <span>ACTIVE</span>
                </div>
              {/if}
            </div>

            <span class="preset-card-title">{preset.name}</span>
            <p class="preset-card-desc">{preset.description}</p>

            <div class="tags-row">
              {#each preset.tags as tag}
                <span class="tag-pill">#{tag}</span>
              {/each}
            </div>
          </button>
        {:else}
          <div class="no-results">
            <span>No presets match your search query</span>
          </div>
        {/each}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(7, 9, 13, 0.78);
    backdrop-filter: blur(8px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal-window {
    width: 780px;
    max-height: 80vh;
    background: var(--color-surface-1);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-depth-lg);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    user-select: none;
    font-family: var(--font-sans);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    background: var(--color-surface-2);
    border-bottom: 1px solid var(--color-border-default);
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .modal-title {
    font-size: 13px;
    font-weight: 800;
    color: var(--color-text-primary);
    letter-spacing: 0.5px;
  }

  .close-btn {
    background: transparent;
    border: 1px solid transparent;
    color: var(--color-text-secondary);
    cursor: pointer;
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-sm);
    transition: color var(--transition-fast), background-color var(--transition-fast), border-color var(--transition-fast);
  }
  .close-btn:hover {
    color: var(--color-text-primary);
    background-color: var(--color-surface-3);
    border-color: var(--color-border-bright);
  }

  .modal-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    background: var(--color-surface-2);
    border-bottom: 1px solid var(--color-border-default);
    gap: 16px;
  }

  .category-tabs {
    display: flex;
    gap: 4px;
  }

  .cat-btn {
    padding: 6px 12px;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: color var(--transition-fast), background-color var(--transition-fast), border-color var(--transition-fast);
  }
  .cat-btn:hover {
    color: var(--color-text-primary);
    background: var(--color-surface-hover);
    border-color: var(--color-border-bright);
  }
  .cat-btn.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand);
    color: var(--color-accent-brand);
  }

  .search-box {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    padding: 6px 12px;
    transition: border-color var(--transition-fast);
  }
  .search-box:focus-within {
    border-color: var(--color-accent-brand);
  }

  .search-box input {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: var(--color-text-primary);
    font-size: 12px;
    font-family: var(--font-sans);
  }
  .search-box input::placeholder {
    color: var(--color-text-muted);
  }

  .preset-grid {
    padding: 16px 20px;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    overflow-y: auto;
    max-height: 520px;
  }

  .preset-card {
    background: var(--color-surface-2);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    padding: 14px;
    cursor: pointer;
    transition: background-color var(--transition-fast), border-color var(--transition-fast);
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-align: left;
  }
  .preset-card:hover {
    background: var(--color-surface-hover);
    border-color: var(--color-border-bright);
  }
  .preset-card.active {
    border-color: var(--color-accent-brand);
    background: var(--color-accent-brand-subtle);
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .category-badge {
    font-size: 9px;
    font-weight: 800;
    color: var(--color-text-muted);
    letter-spacing: 0.5px;
  }

  .active-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-weight: 800;
    color: var(--color-accent-brand);
  }

  .preset-card-title {
    font-size: 13px;
    font-weight: 700;
    color: var(--color-text-primary);
  }

  .preset-card-desc {
    font-size: 11px;
    color: var(--color-text-secondary);
    line-height: 1.4;
    margin: 0;
  }

  .tags-row {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 4px;
  }

  .tag-pill {
    font-size: 10px;
    color: var(--color-text-muted);
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-subtle);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
  }

  .no-results {
    grid-column: span 2;
    text-align: center;
    padding: 40px;
    color: var(--color-text-muted);
    font-style: italic;
  }
</style>
