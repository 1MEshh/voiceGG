<script context="module" lang="ts">
  export interface SelectOption {
    value: string;
    label: string;
    icon?: string;
    badge?: string;
  }
</script>

<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export let value: string = '';
  export let options: SelectOption[] = [];
  export let placeholder: string = 'Select...';
  export let disabled: boolean = false;
  export let size: 'sm' | 'md' = 'md';
  export let onChange: ((val: string) => void) | undefined = undefined;

  let isOpen = false;
  let containerEl: HTMLElement | null = null;
  let focusedIndex = -1;

  $: selectedOption = options.find((o) => o.value === value);

  function toggleOpen() {
    if (disabled) return;
    isOpen = !isOpen;
    if (isOpen) {
      focusedIndex = options.findIndex((o) => o.value === value);
      if (focusedIndex < 0 && options.length > 0) focusedIndex = 0;
    }
  }

  function selectOption(opt: SelectOption) {
    if (disabled) return;
    value = opt.value;
    isOpen = false;
    if (onChange) {
      onChange(opt.value);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (disabled) return;

    if (!isOpen) {
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp' || event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        isOpen = true;
        focusedIndex = options.findIndex((o) => o.value === value);
        if (focusedIndex < 0 && options.length > 0) focusedIndex = 0;
      }
      return;
    }

    if (event.key === 'Escape' || event.key === 'Tab') {
      isOpen = false;
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      focusedIndex = (focusedIndex + 1) % options.length;
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      focusedIndex = (focusedIndex - 1 + options.length) % options.length;
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      if (focusedIndex >= 0 && focusedIndex < options.length) {
        selectOption(options[focusedIndex]);
      }
    }
  }

  function handleClickOutside(event: MouseEvent) {
    if (containerEl && !containerEl.contains(event.target as Node)) {
      isOpen = false;
    }
  }

  onMount(() => {
    window.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('click', handleClickOutside);
    }
  });
</script>

<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
  class="custom-select-container {size}"
  class:is-open={isOpen}
  class:is-disabled={disabled}
  bind:this={containerEl}
  on:keydown={handleKeyDown}
>
  <button
    type="button"
    class="select-trigger"
    aria-haspopup="listbox"
    aria-expanded={isOpen}
    aria-disabled={disabled}
    disabled={disabled}
    on:click={toggleOpen}
  >
    <div class="selected-content">
      {#if selectedOption?.icon}
        <span class="opt-icon">{selectedOption.icon}</span>
      {/if}
      <span class="opt-label" class:placeholder={!selectedOption}>
        {selectedOption ? selectedOption.label : placeholder}
      </span>
      {#if selectedOption?.badge}
        <span class="opt-badge">{selectedOption.badge}</span>
      {/if}
    </div>
    <svg
      class="chevron"
      class:rotate={isOpen}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <polyline points="6 9 12 15 18 9"></polyline>
    </svg>
  </button>

  {#if isOpen}
    <div class="select-dropdown" role="listbox">
      {#each options as opt, i}
        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <div
          role="option"
          tabindex="-1"
          aria-selected={opt.value === value}
          class="select-option"
          class:is-selected={opt.value === value}
          class:is-focused={i === focusedIndex}
          on:click={() => selectOption(opt)}
        >
          <div class="option-left">
            {#if opt.icon}
              <span class="opt-icon">{opt.icon}</span>
            {/if}
            <span class="option-name">{opt.label}</span>
            {#if opt.badge}
              <span class="opt-badge">{opt.badge}</span>
            {/if}
          </div>
          {#if opt.value === value}
            <svg
              class="check-icon"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2.5"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
          {/if}
        </div>
      {/each}
      {#if options.length === 0}
        <div class="select-empty">No options available</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .custom-select-container {
    position: relative;
    width: 100%;
    user-select: none;
    font-family: var(--font-sans);
  }

  .select-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: var(--color-surface-2, #161b26);
    border: 1px solid var(--color-border-default, #1f2637);
    border-radius: var(--radius-md, 6px);
    color: var(--color-text-primary, #f8fafc);
    cursor: pointer;
    font-size: 0.825rem;
    font-weight: 500;
    transition: all var(--transition-fast, 120ms);
    text-align: left;
    outline: none;
    box-sizing: border-box;
  }

  .sm .select-trigger {
    padding: 6px 10px;
    height: 32px;
    font-size: 0.775rem;
  }

  .md .select-trigger {
    padding: 8px 12px;
    height: 38px;
    font-size: 0.825rem;
  }

  .select-trigger:hover:not(:disabled) {
    background: var(--color-surface-hover, #252d40);
    border-color: var(--color-border-bright, #2a344a);
  }

  .select-trigger:focus-visible {
    border-color: var(--color-accent-brand, #ff5200);
    box-shadow: 0 0 0 2px var(--color-accent-brand-subtle, rgba(255, 82, 0, 0.2));
  }

  .is-open .select-trigger {
    border-color: var(--color-accent-brand, #ff5200);
  }

  .is-disabled .select-trigger {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .selected-content {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    flex: 1;
  }

  .opt-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .opt-label.placeholder {
    color: var(--color-text-muted, #64748b);
  }

  .opt-icon {
    font-size: 0.95rem;
    line-height: 1;
    display: inline-flex;
    align-items: center;
  }

  .opt-badge {
    font-size: 0.65rem;
    font-weight: 600;
    text-transform: uppercase;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(255, 82, 0, 0.15);
    color: var(--color-accent-brand, #ff5200);
    border: 1px solid rgba(255, 82, 0, 0.3);
    letter-spacing: 0.04em;
  }

  .chevron {
    width: 14px;
    height: 14px;
    color: var(--color-text-muted, #64748b);
    transition: transform var(--transition-fast, 120ms);
    flex-shrink: 0;
    margin-inline-start: 8px;
  }

  .chevron.rotate {
    transform: rotate(180deg);
    color: var(--color-accent-brand, #ff5200);
  }

  .select-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: var(--color-surface-1, #11151f);
    border: 1px solid var(--color-border-bright, #2a344a);
    border-radius: var(--radius-md, 6px);
    box-shadow: var(--shadow-depth-lg, 0 6px 20px rgba(0, 0, 0, 0.55));
    max-height: 240px;
    overflow-y: auto;
    z-index: 100;
    padding: 4px;
    box-sizing: border-box;
  }

  .select-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 10px;
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-primary, #f8fafc);
    font-size: 0.8rem;
    font-weight: 500;
    cursor: pointer;
    transition: background var(--transition-fast, 120ms);
  }

  .select-option:hover,
  .select-option.is-focused {
    background: var(--color-surface-hover, #252d40);
    color: #ffffff;
  }

  .select-option.is-selected {
    background: rgba(255, 82, 0, 0.12);
    color: var(--color-accent-brand, #ff5200);
    font-weight: 600;
  }

  .option-left {
    display: flex;
    align-items: center;
    gap: 8px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .option-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .check-icon {
    width: 14px;
    height: 14px;
    color: var(--color-accent-brand, #ff5200);
    flex-shrink: 0;
    margin-inline-start: 8px;
  }

  .select-empty {
    padding: 12px;
    text-align: center;
    font-size: 0.775rem;
    color: var(--color-text-muted, #64748b);
  }

  /* Custom dark scrollbar */
  .select-dropdown::-webkit-scrollbar {
    width: 6px;
  }
  .select-dropdown::-webkit-scrollbar-track {
    background: transparent;
  }
  .select-dropdown::-webkit-scrollbar-thumb {
    background: var(--color-border-bright, #2a344a);
    border-radius: 3px;
  }
  .select-dropdown::-webkit-scrollbar-thumb:hover {
    background: var(--color-neutral-600, #333d56);
  }
</style>
