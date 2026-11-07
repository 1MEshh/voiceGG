<script lang="ts">
  import type { SystemStatus, AudioDevice } from '../types';
  import CustomSelect, { type SelectOption } from './CustomSelect.svelte';
  import {
    X,
    Settings,
    AlertTriangle,
    ShieldCheck,
    Cpu,
    Globe,
    Headphones,
    Minimize2,
  } from '@lucide/svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { t, locale, setLanguage } from './i18n';

  export let isOpen: boolean;
  export let status: SystemStatus;
  export let onClose: () => void;
  export let onPanicReset: () => void;
  export let onToggleAutoDetect: ((enabled: boolean) => void) | undefined = undefined;
  export let onSetQuantum: ((quantum: number) => void) | undefined = undefined;
  export let onSetChannelColor: ((channel: string, color: string) => void) | undefined = undefined;

  // Suppress unused warning while maintaining prop compatibility for callers
  $: if (false) console.log(onSetChannelColor);

  // Reactive state synced with status.config
  $: autoDetect = status?.config?.auto_game_detection ?? true;
  $: closeToTray = (status?.config as any)?.close_to_tray ?? true;
  $: currentQuantum = String(status?.config?.buffer_quantum || 256);

  const quantumOptions: SelectOption[] = [
    { value: '64', label: '64 Samples (1.3 ms — Ultra-Low Latency)' },
    { value: '128', label: '128 Samples (2.6 ms — Competitive Gaming)' },
    { value: '256', label: '256 Samples (5.3 ms — Balanced Pro Audio)' },
    { value: '512', label: '512 Samples (10.6 ms — High Stability)' },
    { value: '1024', label: '1024 Samples (21.3 ms — Maximum Buffer)' },
  ];

  $: playbackDeviceOptions = [
    { value: 'default', label: $t('mixer.default_sink') },
    ...(status?.devices || [])
      .filter((d) => d.device_type.toLowerCase() === 'sink')
      .map((d) => ({ value: d.name, label: d.description || d.name })),
  ];

  $: micDeviceOptions = [
    { value: 'default', label: $t('mixer.default_source') },
    ...(status?.devices || [])
      .filter((d) => d.device_type.toLowerCase() === 'source')
      .map((d) => ({ value: d.name, label: d.description || d.name })),
  ];

  async function handleToggleAutoDetect(enabled: boolean) {
    if (status?.config) {
      status.config.auto_game_detection = enabled;
      status = { ...status };
    }
    onToggleAutoDetect?.(enabled);
    try {
      await invoke('set_auto_game_detection', { enabled });
    } catch (e) {
      console.error('Failed to set auto game detection:', e);
    }
  }

  async function handleSetCloseToTray(enabled: boolean) {
    if (status?.config) {
      (status.config as any).close_to_tray = enabled;
      status = { ...status };
    }
    try {
      await invoke('set_close_to_tray', { enabled });
    } catch (e) {
      console.error('Failed to set close to tray behavior:', e);
    }
  }

  async function handleSetAudioDevice(deviceType: 'sink' | 'source', deviceName: string) {
    if (status?.config) {
      if (deviceType === 'sink') {
        status.config.preferred_output_device = deviceName === 'default' ? null : deviceName;
      } else {
        status.config.preferred_input_device = deviceName === 'default' ? null : deviceName;
      }
      status = { ...status };
    }
    try {
      await invoke('set_audio_device', { deviceType, deviceName });
    } catch (e) {
      console.error(`Failed to set ${deviceType} device:`, e);
    }
  }

  async function handleSetQuantum(quantumVal: number) {
    if (status?.config) {
      status.config.buffer_quantum = quantumVal;
      status = { ...status };
    }
    onSetQuantum?.(quantumVal);
    try {
      await invoke('set_quantum', { quantum: quantumVal });
    } catch (e) {
      console.error('Failed to set quantum buffer:', e);
    }
  }

  function handlePanicReset() {
    if (confirm($t('settings.confirm_panic'))) {
      onPanicReset();
    }
  }

  async function handleQuit() {
    if (confirm($t('settings.confirm_quit'))) {
      try {
        await invoke('quit_app');
      } catch (e) {
        console.error('Failed to quit app:', e);
      }
    }
  }
</script>

<svelte:window on:keydown={(e) => { if (isOpen && e.key === 'Escape') onClose(); }} />

{#if isOpen}
  <div
    class="modal-backdrop"
    role="presentation"
    on:click={onClose}
  >
    <div
      class="modal-window"
      role="dialog"
      aria-modal="true"
      aria-label={$t('settings.modal_aria')}
      tabindex="-1"
      on:click|stopPropagation
      on:keydown|stopPropagation
    >
      <div class="modal-header">
        <div class="header-left">
          <Settings size={18} color="var(--color-accent-brand)" />
          <span class="modal-title">{$t('settings.title')}</span>
        </div>
        <button class="close-btn" aria-label={$t('settings.close')} on:click={onClose}>
          <X size={18} />
        </button>
      </div>

      <div class="modal-body">
        <!-- Language Selection Section -->
        <div class="setting-section">
          <div class="section-title-row">
            <Globe size={16} color="var(--color-accent-brand)" />
            <span class="section-title">{$t('settings.language_section')}</span>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">{$t('settings.language')}</span>
              <span class="card-desc">{$t('settings.language_desc')}</span>
            </div>
            <div class="lang-selector">
              <button
                type="button"
                class="lang-pill"
                class:active={$locale === 'en'}
                aria-pressed={$locale === 'en'}
                on:click={() => setLanguage('en')}
              >
                {$t('settings.english')}
              </button>
              <button
                type="button"
                class="lang-pill"
                class:active={$locale === 'ar'}
                aria-pressed={$locale === 'ar'}
                on:click={() => setLanguage('ar')}
              >
                {$t('settings.arabic')}
              </button>
            </div>
          </div>
        </div>

        <!-- Window Close Behavior Section -->
        <div class="setting-section">
          <div class="section-title-row">
            <Minimize2 size={16} color="var(--color-accent-info)" />
            <span class="section-title">{$t('settings.close_behavior')}</span>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">{$t('settings.close_behavior')}</span>
              <span class="card-desc">{$t('settings.close_behavior_desc')}</span>
            </div>
            <div class="lang-selector">
              <button
                type="button"
                class="lang-pill"
                class:active={closeToTray}
                aria-pressed={closeToTray}
                on:click={() => handleSetCloseToTray(true)}
              >
                {$t('settings.close_to_tray')}
              </button>
              <button
                type="button"
                class="lang-pill"
                class:active={!closeToTray}
                aria-pressed={!closeToTray}
                on:click={() => handleSetCloseToTray(false)}
              >
                {$t('settings.close_and_quit')}
              </button>
            </div>
          </div>
        </div>

        <!-- Hardware Audio Devices Section -->
        <div class="setting-section">
          <div class="section-title-row">
            <Headphones size={16} color="var(--color-accent-brand)" />
            <span class="section-title">{$t('settings.devices_section')}</span>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">{$t('mixer.playback_device')}</span>
              <span class="card-desc">{$t('settings.playback_device_desc')}</span>
            </div>
            <div class="device-select-wrapper">
              <CustomSelect
                value={status?.config?.preferred_output_device || 'default'}
                options={playbackDeviceOptions}
                size="sm"
                onChange={(val) => handleSetAudioDevice('sink', val)}
              />
            </div>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">{$t('mixer.microphone')}</span>
              <span class="card-desc">{$t('settings.mic_device_desc')}</span>
            </div>
            <div class="device-select-wrapper">
              <CustomSelect
                value={status?.config?.preferred_input_device || 'default'}
                options={micDeviceOptions}
                size="sm"
                onChange={(val) => handleSetAudioDevice('source', val)}
              />
            </div>
          </div>
        </div>

        <!-- Audio Safety & Quantum Section -->
        <div class="setting-section">
          <div class="section-title-row">
            <ShieldCheck size={16} color="var(--color-accent-success)" />
            <span class="section-title">{$t('settings.audio_safety')}</span>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">{$t('settings.limiter_title')}</span>
              <span class="card-desc">{$t('settings.limiter_desc')}</span>
            </div>
            <span class="badge active-badge">{$t('settings.mandatory_active')}</span>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">{$t('settings.low_latency_quantum')}</span>
              <span class="card-desc">{$t('settings.low_latency_desc')}</span>
            </div>
            <div class="quantum-select-wrapper">
              <CustomSelect
                value={currentQuantum}
                options={quantumOptions}
                size="sm"
                onChange={(val) => handleSetQuantum(Number(val))}
              />
            </div>
          </div>
        </div>

        <!-- Auto Game Detection Section -->
        <div class="setting-section">
          <div class="section-title-row">
            <Cpu size={16} color="var(--color-accent-info)" />
            <span class="section-title">{$t('settings.gaming_integration')}</span>
          </div>
          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">{$t('settings.auto_game_detection')}</span>
              <span class="card-desc">{$t('settings.auto_game_desc')}</span>
            </div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                aria-label={$t('settings.toggle_auto_game_aria')}
                checked={autoDetect}
                on:change={(e) => handleToggleAutoDetect(e.currentTarget.checked)}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
        </div>

        <!-- Emergency Reset & Shutdown -->
        <div class="setting-section danger-section">
          <div class="section-title-row">
            <AlertTriangle size={16} color="var(--color-accent-danger)" />
            <span class="section-title text-red">{$t('settings.emergency_restore')}</span>
          </div>
          <div class="danger-card">
            <div class="card-left">
              <span class="card-label">{$t('settings.reset_audio_graph')}</span>
              <span class="card-desc">{$t('settings.reset_desc')}</span>
            </div>
            <button
              class="danger-btn"
              aria-label={$t('settings.restore_aria')}
              on:click={handlePanicReset}
            >
              {$t('settings.restore_defaults')}
            </button>
          </div>
          <div class="danger-card">
            <div class="card-left">
              <span class="card-label">{$t('settings.quit_title')}</span>
              <span class="card-desc">{$t('settings.quit_desc')}</span>
            </div>
            <button
              class="danger-btn quit-action-btn"
              aria-label={$t('settings.quit_title')}
              on:click={handleQuit}
            >
              {$t('settings.quit_btn')}
            </button>
          </div>
        </div>

        <!-- Version Info -->
        <div class="version-row">
          <span class="version-label">{$t('settings.version')}</span>
          <span class="version-val">{$t('settings.version_value')}</span>
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
    width: 660px;
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
    flex: 1;
    min-width: 0;
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

  .quantum-select-wrapper,
  .device-select-wrapper {
    width: 280px;
    flex-shrink: 0;
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
    flex-shrink: 0;
  }

  .danger-btn:hover {
    background: #DC2626;
  }

  .quit-action-btn {
    background: #dc2626;
    border: 1px solid #b91c1c;
  }

  .quit-action-btn:hover {
    background: #b91c1c;
  }

  .lang-selector {
    display: flex;
    gap: 6px;
    background: var(--color-surface-3);
    padding: 4px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border-default);
    flex-shrink: 0;
  }

  .lang-pill {
    padding: 6px 14px;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: color var(--transition-fast), background-color var(--transition-fast), border-color var(--transition-fast);
    white-space: nowrap;
  }

  .lang-pill:hover {
    color: var(--color-text-primary);
  }

  .lang-pill.active {
    background: var(--color-surface-1);
    color: var(--color-accent-brand);
    border-color: var(--color-border-bright);
    box-shadow: var(--shadow-depth-sm);
  }

  .version-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    background: var(--color-surface-2);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    font-size: 11px;
    color: var(--color-text-muted);
  }

  .version-label {
    font-weight: 700;
  }

  .version-val {
    font-family: var(--font-mono);
    color: var(--color-text-secondary);
  }
</style>
