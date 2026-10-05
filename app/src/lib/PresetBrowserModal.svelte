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
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <div class="modal-header">
        <div class="header-left">
          <Sliders size={18} color="#22c55e" />
          <span class="modal-title">BROWSE EQUALIZER PRESETS</span>
        </div>
        <button class="close-btn" on:click={onClose}>
          <X size={18} />
        </button>
      </div>

      <div class="modal-toolbar">
        <div class="category-tabs">
          <button class="cat-btn" class:active={selectedCategory === 'all'} on:click={() => selectedCategory = 'all'}>ALL</button>
          <button class="cat-btn" class:active={selectedCategory === 'game'} on:click={() => selectedCategory = 'game'}>GAME</button>
          <button class="cat-btn" class:active={selectedCategory === 'chat'} on:click={() => selectedCategory = 'chat'}>CHAT</button>
          <button class="cat-btn" class:active={selectedCategory === 'media'} on:click={() => selectedCategory = 'media'}>MEDIA</button>
          <button class="cat-btn" class:active={selectedCategory === 'mic'} on:click={() => selectedCategory = 'mic'}>MIC</button>
        </div>

        <div class="search-box">
          <Search size={14} color="#64748b" />
          <input
            type="text"
            placeholder="Search game (CS2, Apex...), tags, or sound profile..."
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
            on:click={() => onSelectPreset(preset)}
          >
            <div class="card-top">
              <span class="category-badge">{preset.category.toUpperCase()}</span>
              {#if isActive}
                <div class="active-badge">
                  <Check size={12} color="#22c55e" />
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
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal-window {
    width: 780px;
    max-height: 80vh;
    background: #14181f;
    border: 1px solid #283344;
    border-radius: 10px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.6);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    user-select: none;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    background: #181e28;
    border-bottom: 1px solid #252f3e;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .modal-title {
    font-size: 14px;
    font-weight: 700;
    color: #f1f5f9;
    letter-spacing: 0.5px;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    cursor: pointer;
  }

  .close-btn:hover {
    color: #f8fafc;
  }

  .modal-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    background: #12151c;
    border-bottom: 1px solid #202733;
    gap: 16px;
  }

  .category-tabs {
    display: flex;
    gap: 4px;
  }

  .cat-btn {
    padding: 5px 12px;
    background: #1c232d;
    border: 1px solid #283340;
    border-radius: 5px;
    color: #94a3b8;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
  }

  .cat-btn:hover {
    color: #f1f5f9;
    background: #252f3e;
  }

  .cat-btn.active {
    background: #22c55e1a;
    border-color: #22c55e88;
    color: #4ade80;
  }

  .search-box {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    background: #181d26;
    border: 1px solid #27313f;
    border-radius: 6px;
    padding: 6px 10px;
  }

  .search-box input {
    width: 100%;
    background: transparent;
    border: none;
    outline: none;
    color: #f1f5f9;
    font-size: 12px;
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
    background: #171d26;
    border: 1px solid #242e3d;
    border-radius: 8px;
    padding: 14px;
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .preset-card:hover {
    background: #1e2634;
    border-color: #38475c;
  }

  .preset-card.active {
    border-color: #22c55e;
    background: #14241d;
  }

  .card-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .category-badge {
    font-size: 9px;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.5px;
  }

  .active-badge {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 10px;
    font-weight: 700;
    color: #4ade80;
  }

  .preset-card-title {
    font-size: 13px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .preset-card-desc {
    font-size: 11px;
    color: #94a3b8;
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
    color: #64748b;
    background: #12151c;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .no-results {
    grid-column: span 2;
    text-align: center;
    padding: 40px;
    color: #64748b;
    font-style: italic;
  }
</style>
