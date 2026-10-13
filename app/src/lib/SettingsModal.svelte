<script lang="ts">
  import type { SystemStatus } from '../types';
  import { CHANNELS } from './theme';
  import { X, Settings, AlertTriangle, ShieldCheck, Palette, Cpu } from '@lucide/svelte';

  export let isOpen: boolean;
  export let status: SystemStatus;
  export let onClose: () => void;
  export let onPanicReset: () => void;
  export let onToggleAutoDetect: (enabled: boolean) => void;

  let autoDetect = status.config.auto_game_detection ?? true;
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
      aria-label="Settings and Preferences Modal"
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <div class="modal-header">
        <div class="header-left">
          <Settings size={18} color="var(--color-accent-brand)" />
          <span class="modal-title">SETTINGS & PREFERENCES</span>
        </div>
        <button class="close-btn" aria-label="Close Settings" on:click={onClose}>
          <X size={18} />
        </button>
      </div>

      <div class="modal-body">
        <!-- Audio Safety Section -->
        <div class="setting-section">
          <div class="section-title-row">
            <ShieldCheck size={16} color="var(--color-accent-success)" />
            <span class="section-title">AUDIO ENGINE & SAFETY</span>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">True-Peak Hard Limiter</span>
              <span class="card-desc">Non-bypassable −6 dBFS safety ceiling on Master output to protect hearing and hardware.</span>
            </div>
            <span class="badge active-badge">MANDATORY ACTIVE</span>
          </div>
        </div>

        <!-- Auto Game Detection Section -->
        <div class="setting-section">
          <div class="section-title-row">
            <Cpu size={16} color="var(--color-accent-info)" />
            <span class="section-title">GAMING INTEGRATION</span>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">Automatic Game Process Detection</span>
              <span class="card-desc">Scans native and Wine/Proton games to auto-apply tuning presets and route to Game channel.</span>
            </div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                aria-label="Toggle Automatic Game Process Detection"
                bind:checked={autoDetect}
                on:change={() => onToggleAutoDetect(autoDetect)}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
        </div>

        <!-- Custom Channel Colors -->
        <div class="setting-section">
          <div class="section-title-row">
            <Palette size={16} color="var(--color-channel-media)" />
            <span class="section-title">CHANNEL ACCENT COLORS</span>
          </div>
          <div class="color-pickers-grid">
            {#each Object.entries(CHANNELS) as [id, info]}
              <div class="color-item">
                <span class="color-label">{info.label}</span>
                <input
                  type="color"
                  aria-label="Accent color for {info.label}"
                  value={info.defaultColor}
                  class="color-input"
                />
              </div>
            {/each}
          </div>
        </div>

        <!-- Emergency Reset -->
        <div class="setting-section danger-section">
          <div class="section-title-row">
            <AlertTriangle size={16} color="var(--color-accent-danger)" />
            <span class="section-title text-red">EMERGENCY RESTORE</span>
          </div>
          <div class="danger-card">
            <div class="card-left">
              <span class="card-label">Reset Audio Graph</span>
              <span class="card-desc">Unloads all VoiceGG virtual devices and immediately restores your original physical sinks and sources.</span>
            </div>
            <button
              class="danger-btn"
              aria-label="Restore Audio Defaults"
              on:click={onPanicReset}
            >
              RESTORE DEFAULTS
            </button>
          </div>
        </div>
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
    width: 650px;
    max-height: 85vh;
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

  .modal-body {
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    overflow-y: auto;
  }

  .setting-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .section-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 800;
    color: var(--color-text-secondary);
    letter-spacing: 0.5px;
  }

  .text-red {
    color: var(--color-accent-danger);
  }

  .setting-card, .danger-card {
    background: var(--color-surface-2);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    padding: 14px 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .danger-card {
    border-color: var(--color-accent-danger);
    background: var(--color-accent-danger-subtle);
  }

  .card-left {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .card-label {
    font-size: 13px;
    font-weight: 700;
    color: var(--color-text-primary);
  }

  .card-desc {
    font-size: 11px;
    color: var(--color-text-secondary);
    line-height: 1.4;
  }

  .badge {
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-weight: 700;
    font-family: var(--font-mono);
    white-space: nowrap;
  }

  .active-badge {
    background: var(--color-accent-success-subtle);
    border: 1px solid var(--color-accent-success);
    color: var(--color-accent-success);
  }

  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
    flex-shrink: 0;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0; left: 0; right: 0; bottom: 0;
    background-color: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    transition: background-color var(--transition-fast), border-color var(--transition-fast);
    border-radius: 20px;
  }

  .toggle-slider:before {
    position: absolute;
    content: "";
    height: 14px;
    width: 14px;
    left: 2px;
    bottom: 2px;
    background-color: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    transition: transform var(--transition-fast);
    border-radius: 50%;
  }

  input:checked + .toggle-slider {
    background-color: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand);
  }

  input:checked + .toggle-slider:before {
    transform: translateX(16px);
    background-color: var(--color-accent-brand);
    border-color: var(--color-accent-brand);
  }

  .color-pickers-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }

  .color-item {
    background: var(--color-surface-2);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    padding: 8px 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .color-label {
    font-size: 11px;
    font-weight: 700;
    color: var(--color-text-secondary);
  }

  .color-input {
    border: 1px solid var(--color-border-bright);
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    background: transparent;
  }

  .danger-btn {
    padding: 8px 16px;
    min-height: 36px;
    background: var(--color-accent-danger);
    border: none;
    border-radius: var(--radius-sm);
    color: white;
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    white-space: nowrap;
    transition: background-color var(--transition-fast);
  }

  .danger-btn:hover {
    background: #DC2626;
  }
</style>
