<script lang="ts">
  import type { ChannelId, SystemStatus } from '../types';
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
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <div class="modal-header">
        <div class="header-left">
          <Settings size={18} color="#94a3b8" />
          <span class="modal-title">SETTINGS & PREFERENCES</span>
        </div>
        <button class="close-btn" on:click={onClose}>
          <X size={18} />
        </button>
      </div>

      <div class="modal-body">
        <!-- Audio Safety Section -->
        <div class="setting-section">
          <div class="section-title-row">
            <ShieldCheck size={16} color="#22c55e" />
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
            <Cpu size={16} color="#38bdf8" />
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
            <Palette size={16} color="#ec4899" />
            <span class="section-title">CHANNEL ACCENT COLORS</span>
          </div>
          <div class="color-pickers-grid">
            {#each Object.entries(CHANNELS) as [id, info]}
              <div class="color-item">
                <span class="color-label">{info.label}</span>
                <input type="color" value={info.defaultColor} class="color-input" />
              </div>
            {/each}
          </div>
        </div>

        <!-- Emergency Reset -->
        <div class="setting-section danger-section">
          <div class="section-title-row">
            <AlertTriangle size={16} color="#ef4444" />
            <span class="section-title text-red">EMERGENCY RESTORE</span>
          </div>
          <div class="danger-card">
            <div class="card-left">
              <span class="card-label">Reset Audio Graph</span>
              <span class="card-desc">Unloads all VoiceGG virtual devices and immediately restores your original physical sinks and sources.</span>
            </div>
            <button class="danger-btn" on:click={onPanicReset}>
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
    background: rgba(0, 0, 0, 0.75);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }

  .modal-window {
    width: 650px;
    max-height: 85vh;
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
    font-weight: 700;
    color: #94a3b8;
    letter-spacing: 0.5px;
  }

  .text-red {
    color: #ef4444;
  }

  .setting-card, .danger-card {
    background: #181d26;
    border: 1px solid #26303f;
    border-radius: 8px;
    padding: 12px 16px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .danger-card {
    border-color: #ef444444;
    background: #201316;
  }

  .card-left {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .card-label {
    font-size: 13px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .card-desc {
    font-size: 11px;
    color: #94a3b8;
    line-height: 1.4;
  }

  .badge {
    padding: 4px 8px;
    border-radius: 6px;
    font-size: 10px;
    font-weight: 700;
    font-family: monospace;
    white-space: nowrap;
  }

  .active-badge {
    background: #22c55e22;
    border: 1px solid #22c55e66;
    color: #4ade80;
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
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: #242c38;
    transition: .2s;
    border-radius: 20px;
  }

  .toggle-slider:before {
    position: absolute;
    content: "";
    height: 14px;
    width: 14px;
    left: 3px;
    bottom: 3px;
    background-color: white;
    transition: .2s;
    border-radius: 50%;
  }

  input:checked + .toggle-slider {
    background-color: #38bdf8;
  }

  input:checked + .toggle-slider:before {
    transform: translateX(16px);
  }

  .color-pickers-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }

  .color-item {
    background: #181d26;
    border: 1px solid #26303f;
    border-radius: 6px;
    padding: 8px 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .color-label {
    font-size: 11px;
    font-weight: 700;
    color: #cbd5e1;
  }

  .color-input {
    border: none;
    width: 24px;
    height: 24px;
    border-radius: 4px;
    cursor: pointer;
    background: transparent;
  }

  .danger-btn {
    padding: 7px 14px;
    background: #ef4444;
    border: none;
    border-radius: 6px;
    color: white;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.15s ease;
  }

  .danger-btn:hover {
    background: #dc2626;
  }
</style>
