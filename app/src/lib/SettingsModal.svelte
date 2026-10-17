<script lang="ts">
  import type { SystemStatus, ChannelId } from '../types';
  import { CHANNELS } from './theme';
  import CustomSelect, { type SelectOption } from './CustomSelect.svelte';
  import { X, Settings, AlertTriangle, ShieldCheck, Palette, Cpu, Globe, Hash } from '@lucide/svelte';
  import { t, locale, numerals, setLanguage } from './i18n';

  export let isOpen: boolean;
  export let status: SystemStatus;
  export let onClose: () => void;
  export let onPanicReset: () => void;
  export let onToggleAutoDetect: (enabled: boolean) => void;
  export let onSetQuantum: ((quantum: number) => void) | undefined = undefined;
  export let onSetChannelColor: ((channel: string, color: string) => void) | undefined = undefined;

  let autoDetect = status.config.auto_game_detection ?? true;

  const quantumOptions: SelectOption[] = [
    { value: '64', label: '64 Samples (1.3 ms — Ultra-Low Latency)' },
    { value: '128', label: '128 Samples (2.6 ms — Competitive Gaming)' },
    { value: '256', label: '256 Samples (5.3 ms — Balanced Pro Audio)' },
    { value: '512', label: '512 Samples (10.6 ms — High Stability)' },
    { value: '1024', label: '1024 Samples (21.3 ms — Maximum Buffer)' },
  ];

  $: currentQuantum = String(status.config.buffer_quantum || 256);

  function getChannelColor(chId: string, defaultColor: string): string {
    return status.config.theme?.channels?.[chId as ChannelId]?.accent_hex || defaultColor;
  }
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

          <div class="setting-card">
            <div class="card-left">
              <span class="card-label">{$t('settings.numerals')}</span>
              <span class="card-desc">{$t('settings.numerals_desc')}</span>
            </div>
            <div class="lang-selector">
              <button
                type="button"
                class="lang-pill"
                class:active={$numerals === 'western'}
                aria-pressed={$numerals === 'western'}
                on:click={() => numerals.set('western')}
              >
                {$t('settings.numerals_western')}
              </button>
              <button
                type="button"
                class="lang-pill"
                class:active={$numerals === 'eastern'}
                aria-pressed={$numerals === 'eastern'}
                on:click={() => numerals.set('eastern')}
              >
                {$t('settings.numerals_eastern')}
              </button>
            </div>
          </div>
        </div>

        <!-- Audio Safety Section -->
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
                onChange={(val) => onSetQuantum?.(Number(val))}
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
            <span class="section-title">{$t('settings.channel_colors')}</span>
          </div>
          <div class="color-pickers-grid">
            {#each Object.entries(CHANNELS) as [id, info]}
              {@const label = $t(`channels.${id}`) || info.label}
              {@const color = getChannelColor(id, info.defaultColor)}
              <div class="color-item">
                <span class="color-label">{label}</span>
                <div class="color-input-wrapper">
                  <input
                    type="color"
                    aria-label={$t('settings.color_aria', { channel: label })}
                    value={color}
                    class="color-input"
                    on:input={(e) => onSetChannelColor?.(id, e.currentTarget.value)}
                  />
                  <span class="color-hex tabular-nums">{color.toUpperCase()}</span>
                </div>
              </div>
            {/each}
          </div>
        </div>

        <!-- Emergency Reset -->
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
              on:click={onPanicReset}
            >
              {$t('settings.restore_defaults')}
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

  .quantum-select-wrapper {
    width: 260px;
  }

  .color-input-wrapper {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .color-hex {
    font-size: 10px;
    font-weight: 700;
    color: var(--color-text-muted);
    font-family: var(--font-mono);
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

  .lang-selector {
    display: flex;
    gap: 6px;
    background: var(--color-surface-3);
    padding: 4px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border-default);
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
