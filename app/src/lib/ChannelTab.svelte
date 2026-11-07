<script lang="ts">
  import { onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { ChannelId, Preset, ChannelMeter } from '../types';
  import { CHANNELS } from './theme';
  import EqGraph from './EqGraph.svelte';
  import SonarSlider from './SonarSlider.svelte';
  import { Search, Sliders, Sparkles, Volume2, BookmarkPlus, RotateCcw } from '@lucide/svelte';
  import { t } from './i18n';

  export let channel: ChannelId;
  export let currentPreset: Preset;
  export let allPresets: Preset[];
  export let builtinPresets: Preset[] = [];
  export let meter: ChannelMeter | undefined = undefined;
  export let onApplyPreset: (preset: Preset) => void;
  export let onSelectPreset: ((preset: Preset) => void) | undefined = undefined;
  export let onResetPreset: ((channel: ChannelId, presetId: string) => void) | undefined = undefined;
  export let onOpenBrowser: () => void;
  export let onSaveCustomPreset: ((channel: ChannelId, preset: Preset) => void) | undefined = undefined;

  $: info = CHANNELS[channel];

  let isSavingCustom = false;
  let customPresetName = '';

  // Local working copy of the active preset for zero-snap 60fps editing
  let localPreset: Preset = clonePreset(currentPreset);
  let lastIncomingPresetId = currentPreset?.id || '';
  let lastLocalEditAt = 0;
  let saveThrottleTimer: ReturnType<typeof setTimeout> | null = null;
  let lastSaveAt = 0;

  function clonePreset(p: Preset): Preset {
    const copy: Preset = JSON.parse(JSON.stringify(p));
    if (!copy.spatial) copy.spatial = { enabled: false, immersion_pct: 50 };
    if (!copy.smart_volume) copy.smart_volume = { enabled: false, mode: 'balanced' };
    return copy;
  }

  $: if (
    currentPreset &&
    (currentPreset.id !== lastIncomingPresetId || Date.now() - lastLocalEditAt > 600)
  ) {
    lastIncomingPresetId = currentPreset.id;
    localPreset = clonePreset(currentPreset);
  }

  $: factoryPreset = builtinPresets.find((bp) => bp.id === localPreset.id);
  $: isModified = Boolean(
    factoryPreset &&
      (JSON.stringify(localPreset.eq) !== JSON.stringify(factoryPreset.eq) ||
        JSON.stringify(localPreset.spatial) !== JSON.stringify(factoryPreset.spatial) ||
        JSON.stringify(localPreset.smart_volume) !== JSON.stringify(factoryPreset.smart_volume))
  );

  async function handleResetToFactory() {
    if (factoryPreset) {
      localPreset = clonePreset(factoryPreset);
      lastIncomingPresetId = factoryPreset.id;
    }
    if (onResetPreset) {
      onResetPreset(channel, localPreset.id);
    } else {
      try {
        await invoke('reset_preset', { channel, presetId: localPreset.id });
      } catch {
        if (factoryPreset) onApplyPreset(clonePreset(factoryPreset));
      }
    }
  }

  function scheduleSavePreset(immediate = false) {
    lastLocalEditAt = Date.now();
    const now = Date.now();
    if (immediate || now - lastSaveAt >= 40) {
      if (saveThrottleTimer) {
        clearTimeout(saveThrottleTimer);
        saveThrottleTimer = null;
      }
      lastSaveAt = now;
      onApplyPreset(clonePreset(localPreset));
    } else if (!saveThrottleTimer) {
      const remaining = 40 - (now - lastSaveAt);
      saveThrottleTimer = setTimeout(() => {
        saveThrottleTimer = null;
        lastSaveAt = Date.now();
        onApplyPreset(clonePreset(localPreset));
      }, remaining);
    }
  }

  onDestroy(() => {
    if (saveThrottleTimer) clearTimeout(saveThrottleTimer);
  });

  function handleToneChange(type: 'bass' | 'voice' | 'treble', val: number) {
    if (type === 'bass') localPreset.eq.bass_db = val;
    if (type === 'voice') localPreset.eq.voice_db = val;
    if (type === 'treble') localPreset.eq.treble_db = val;
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleToggleBypass() {
    localPreset.eq.enabled = !localPreset.eq.enabled;
    localPreset = { ...localPreset };
    scheduleSavePreset(true);
  }

  function openSaveCustomModal() {
    customPresetName = localPreset.name + ' (Custom)';
    isSavingCustom = true;
  }

  function handleConfirmSaveCustom() {
    const trimmed = customPresetName.trim();
    if (!trimmed) return;
    const newPreset: Preset = {
      ...clonePreset(localPreset),
      id: `custom_${Date.now()}`,
      name: trimmed,
      description: `Custom tuned preset for ${channel}`,
      category: 'custom',
      tags: [channel, 'custom'],
    };
    if (onSaveCustomPreset) {
      onSaveCustomPreset(channel, newPreset);
    }
    isSavingCustom = false;
  }

  // Filter presets for this channel's category, excluding mic presets
  $: categoryPresets = allPresets.filter((p) => {
    if (p.category === channel) return true;
    if (p.category === 'custom') {
      if (p.tags?.includes('mic')) return false;
      if (p.description?.toLowerCase().includes('mic') || p.description?.toLowerCase().includes('microphone')) return false;
      if (p.noise_gate) return false;
      if (p.tags && p.tags.some(t => ['game', 'chat', 'media', 'aux'].includes(t))) {
        return p.tags.includes(channel);
      }
      return true;
    }
    return false;
  });
</script>

<div class="channel-view">
  <!-- Preset Bar -->
  <div class="preset-bar">
    <div class="preset-info">
      <div class="preset-icon-badge">
        <Sliders size={18} color={info.defaultColor} />
      </div>
      <div class="preset-titles">
        <div class="preset-title-row">
          <span class="preset-name">{localPreset.name}</span>
          <span class="preset-ch-tag" style="color: {info.defaultColor}; border-color: {info.defaultColor}44;">
            {info.label}
          </span>
        </div>
        <span class="preset-desc">{localPreset.description}</span>
      </div>
    </div>

    <div class="preset-actions-right">
      {#if isModified}
        <button
          type="button"
          class="reset-preset-btn"
          on:click={handleResetToFactory}
          title="Reset to factory preset values"
        >
          <RotateCcw size={14} />
          <span>Reset</span>
        </button>
      {/if}

      <button
        type="button"
        class="save-custom-btn"
        on:click={openSaveCustomModal}
        title="Save current settings as new custom preset"
      >
        <BookmarkPlus size={14} />
        <span>Save Custom</span>
      </button>

      <button
        type="button"
        class="browse-btn"
        aria-label={$t('channel_tab.browse_presets_aria', { channel: $t(`channels.${channel}`) })}
        on:click={onOpenBrowser}
      >
        <Search size={14} />
        <span>{$t('channel_tab.browse_presets')}</span>
      </button>
    </div>
  </div>

  <!-- Preset Quick-Select Strip -->
  <div class="favorites-strip">
    <span class="fav-label">PRESETS:</span>
    <div class="fav-slots">
      {#each categoryPresets.slice(0, 7) as favPreset, idx}
        <button
          type="button"
          class="fav-pill"
          class:active={favPreset.id === localPreset.id}
          style={favPreset.id === localPreset.id
            ? `border-color: ${info.defaultColor}; color: ${info.defaultColor}; background: ${info.defaultColor}14;`
            : ''}
          aria-label={$t('channel_tab.select_favorite', { idx: idx + 1, name: favPreset.name })}
          on:click={() => {
            lastIncomingPresetId = favPreset.id;
            localPreset = clonePreset(favPreset);
            if (onSelectPreset) {
              onSelectPreset(favPreset);
            } else {
              onApplyPreset(favPreset);
            }
          }}
        >
          <span class="fav-idx tabular-nums">{idx + 1}</span>
          <span class="fav-title">{favPreset.name}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Main Content Grid -->
  <div class="cards-grid">
    <!-- Equalizer Card -->
    <div class="card eq-card">
      <div class="card-header">
        <div class="card-title">
          <Sliders size={15} color={info.defaultColor} />
          <span>{$t('channel_tab.parametric_eq')}</span>
        </div>
        <div class="card-actions">
          <button
            type="button"
            class="bypass-btn"
            class:bypassed={!localPreset.eq.enabled}
            on:click={handleToggleBypass}
            title={localPreset.eq.enabled ? $t('channel_tab.bypass_eq') : $t('channel_tab.enable_eq')}
            aria-label={localPreset.eq.enabled ? $t('channel_tab.bypass_eq') : $t('channel_tab.enable_eq')}
          >
            {!localPreset.eq.enabled ? $t('channel_tab.eq_bypassed') : $t('channel_tab.bypass_eq')}
          </button>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('channel_tab.toggle_eq_aria')}
              bind:checked={localPreset.eq.enabled}
              on:change={() => {
                localPreset = { ...localPreset };
                scheduleSavePreset(true);
              }}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
      </div>

      <!-- Canvas Graph with Sleek Single-Band Inline Inspector -->
      <EqGraph
        eqConfig={localPreset.eq}
        {meter}
        accentColor={info.defaultColor}
        onChange={(eq) => {
          localPreset.eq = eq;
          localPreset = { ...localPreset };
          scheduleSavePreset(false);
        }}
      />

      <!-- Quick Tone Sliders -->
      <div class="quick-tone-row">
        <div class="tone-slider-group">
          <SonarSlider
            label={$t('channel_tab.bass_depth')}
            value={localPreset.eq.bass_db}
            min={-12}
            max={12}
            step={0.5}
            unit="dB"
            centerDetent={true}
            defaultValue={0}
            accentColor={info.defaultColor}
            onChange={(v) => handleToneChange('bass', v)}
          />
        </div>

        <div class="tone-slider-group">
          <SonarSlider
            label={$t('channel_tab.vocal_clarity')}
            value={localPreset.eq.voice_db}
            min={-12}
            max={12}
            step={0.5}
            unit="dB"
            centerDetent={true}
            defaultValue={0}
            accentColor={info.defaultColor}
            onChange={(v) => handleToneChange('voice', v)}
          />
        </div>

        <div class="tone-slider-group">
          <SonarSlider
            label={$t('channel_tab.crisp_air')}
            value={localPreset.eq.treble_db}
            min={-12}
            max={12}
            step={0.5}
            unit="dB"
            centerDetent={true}
            defaultValue={0}
            accentColor={info.defaultColor}
            onChange={(v) => handleToneChange('treble', v)}
          />
        </div>
      </div>
    </div>

    <!-- Channel Specific Cards -->
    {#if channel === 'game' && localPreset.spatial}
      <div class="card">
        <div class="card-header">
          <div class="card-title">
            <Sparkles size={15} color="var(--color-channel-game, #00E599)" />
            <span>{$t('channel_tab.spatial_audio')}</span>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('channel_tab.toggle_spatial_aria')}
              bind:checked={localPreset.spatial.enabled}
              on:change={() => {
                localPreset = { ...localPreset };
                scheduleSavePreset(true);
              }}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
        <p class="card-info">
          {$t('channel_tab.spatial_audio_desc')}
        </p>
        <div class="control-row">
          <SonarSlider
            label={$t('channel_tab.immersion_distance')}
            value={localPreset.spatial.immersion_pct}
            min={0}
            max={100}
            step={1}
            unit="%"
            accentColor="var(--color-channel-game, #00E599)"
            disabled={!localPreset.spatial.enabled}
            onChange={(v) => {
              if (localPreset.spatial) {
                localPreset.spatial.immersion_pct = v;
                localPreset = { ...localPreset };
                scheduleSavePreset(false);
              }
            }}
          />
        </div>
      </div>

      {#if localPreset.smart_volume}
        <div class="card">
          <div class="card-header">
            <div class="card-title">
              <Volume2 size={15} color="var(--color-channel-game, #00E599)" />
              <span>{$t('channel_tab.volume_normalizer')}</span>
            </div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                bind:checked={localPreset.smart_volume.enabled}
                on:change={() => {
                  localPreset = { ...localPreset };
                  scheduleSavePreset(true);
                }}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <p class="card-info">
            {$t('channel_tab.volume_normalizer_desc')}
          </p>
          <div class="radio-group" class:disabled={!localPreset.smart_volume.enabled}>
            <button
              type="button"
              class="pill-btn"
              class:active={localPreset.smart_volume.mode === 'quiet'}
              on:click={() => {
                if (localPreset.smart_volume) {
                  localPreset.smart_volume.mode = 'quiet';
                  localPreset.smart_volume.enabled = true;
                  localPreset = { ...localPreset };
                  scheduleSavePreset(true);
                }
              }}
            >{$t('channel_tab.quiet')}</button>
            <button
              type="button"
              class="pill-btn"
              class:active={localPreset.smart_volume.mode === 'balanced'}
              on:click={() => {
                if (localPreset.smart_volume) {
                  localPreset.smart_volume.mode = 'balanced';
                  localPreset.smart_volume.enabled = true;
                  localPreset = { ...localPreset };
                  scheduleSavePreset(true);
                }
              }}
            >{$t('channel_tab.balanced')}</button>
            <button
              type="button"
              class="pill-btn"
              class:active={localPreset.smart_volume.mode === 'loud'}
              on:click={() => {
                if (localPreset.smart_volume) {
                  localPreset.smart_volume.mode = 'loud';
                  localPreset.smart_volume.enabled = true;
                  localPreset = { ...localPreset };
                  scheduleSavePreset(true);
                }
              }}
            >{$t('channel_tab.loud')}</button>
          </div>
        </div>
      {/if}
    {/if}
  </div>

  {#if isSavingCustom}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="custom-modal-backdrop" on:click={() => (isSavingCustom = false)}>
      <div class="custom-modal-box" on:click|stopPropagation>
        <div class="modal-header">
          <BookmarkPlus size={18} color="var(--color-accent-brand, #FF5200)" />
          <h3>Save Custom Preset</h3>
        </div>
        <p class="modal-desc">Save current parametric EQ curve and DSP tuning into your custom preset library.</p>
        <input
          type="text"
          class="custom-name-input"
          placeholder="Preset Name"
          bind:value={customPresetName}
          on:keydown={(e) => {
            if (e.key === 'Enter') handleConfirmSaveCustom();
            if (e.key === 'Escape') isSavingCustom = false;
          }}
        />
        <div class="modal-btn-row">
          <button type="button" class="btn-cancel" on:click={() => (isSavingCustom = false)}>Cancel</button>
          <button type="button" class="btn-confirm" on:click={handleConfirmSaveCustom}>Save Preset</button>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .channel-view {
    height: 100%;
    overflow-y: auto;
    padding: 20px 24px;
    background: var(--color-bg-app, #0C0E12);
    display: flex;
    flex-direction: column;
    gap: 14px;
    user-select: none;
    font-family: var(--font-sans);
  }

  .preset-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--color-surface-1, #12151B);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-md, 6px);
    padding: 12px 18px;
  }

  .preset-info {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .preset-icon-badge {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-sm, 4px);
    background: var(--color-surface-2, #171B22);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-border-default, #232834);
  }

  .preset-titles {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preset-title-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .preset-name {
    font-size: 14px;
    font-weight: 800;
    color: var(--color-text-primary, #F8FAFC);
  }

  .preset-ch-tag {
    font-size: 9px;
    font-weight: 800;
    font-family: var(--font-mono);
    padding: 1px 6px;
    border-radius: var(--radius-xs, 3px);
    border: 1px solid;
    letter-spacing: 0.5px;
  }

  .preset-desc {
    font-size: 11px;
    color: var(--color-text-secondary, #94A3B8);
  }

  .preset-actions-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .browse-btn,
  .save-custom-btn,
  .reset-preset-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 12px;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-primary, #F8FAFC);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .reset-preset-btn {
    border-color: rgba(255, 82, 0, 0.4);
    color: var(--color-accent-brand, #FF5200);
  }

  .browse-btn:hover,
  .save-custom-btn:hover {
    background: var(--color-surface-hover, #262C38);
    border-color: var(--color-border-bright, #2F3646);
  }

  .reset-preset-btn:hover {
    background: rgba(255, 82, 0, 0.12);
    border-color: var(--color-accent-brand, #FF5200);
  }

  .custom-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(8, 9, 12, 0.78);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .custom-modal-box {
    background: var(--color-surface-1, #12151B);
    border: 1px solid var(--color-border-bright, #2F3646);
    border-radius: var(--radius-md, 6px);
    box-shadow: var(--shadow-depth-lg);
    width: 380px;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .modal-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
    color: var(--color-text-primary, #F8FAFC);
  }

  .modal-desc {
    font-size: 0.775rem;
    color: var(--color-neutral-400, #94A3B8);
    margin: 0;
    line-height: 1.4;
  }

  .custom-name-input {
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    padding: 8px 12px;
    color: var(--color-text-primary, #F8FAFC);
    font-size: 0.85rem;
    font-family: var(--font-sans);
    outline: none;
  }

  .custom-name-input:focus {
    border-color: var(--color-accent-brand, #FF5200);
  }

  .modal-btn-row {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  .btn-cancel {
    padding: 6px 12px;
    border-radius: var(--radius-sm, 4px);
    background: transparent;
    border: 1px solid var(--color-border-default, #232834);
    color: var(--color-neutral-400, #94A3B8);
    font-size: 0.775rem;
    font-weight: 600;
    cursor: pointer;
  }

  .btn-confirm {
    padding: 6px 14px;
    border-radius: var(--radius-sm, 4px);
    background: var(--color-accent-brand, #FF5200);
    border: none;
    color: #ffffff;
    font-size: 0.775rem;
    font-weight: 700;
    cursor: pointer;
  }

  .favorites-strip {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .fav-label {
    font-size: 10px;
    font-weight: 800;
    color: var(--color-text-muted, #64748B);
    letter-spacing: 0.6px;
  }

  .fav-slots {
    display: flex;
    align-items: center;
    gap: 6px;
    overflow-x: auto;
  }

  .fav-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 11px;
    background: var(--color-surface-1, #12151B);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 11px;
    font-weight: 600;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .fav-pill:hover {
    color: var(--color-text-primary, #F8FAFC);
    background: var(--color-surface-hover, #262C38);
    border-color: var(--color-border-bright, #2F3646);
  }

  .fav-idx {
    font-weight: 800;
    font-family: var(--font-mono);
    opacity: 0.6;
  }

  .cards-grid {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .card {
    background: var(--color-surface-1, #12151B);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-md, 6px);
    padding: 16px 18px;
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }

  .card-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    font-weight: 800;
    color: var(--color-text-primary, #F8FAFC);
    letter-spacing: 0.6px;
    text-transform: uppercase;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .bypass-btn {
    padding: 4px 10px;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 10px;
    font-weight: 800;
    font-family: var(--font-mono);
    cursor: pointer;
    letter-spacing: 0.5px;
    transition: all var(--transition-fast);
  }

  .bypass-btn:hover {
    color: var(--color-text-primary, #F8FAFC);
    border-color: var(--color-border-bright, #2F3646);
  }

  .bypass-btn.bypassed {
    background: var(--color-accent-danger-subtle);
    border-color: var(--color-accent-danger, #EF4444);
    color: var(--color-accent-danger, #EF4444);
  }

  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-slider {
    position: absolute;
    cursor: pointer;
    inset: 0;
    background-color: var(--color-surface-3, #1E232D);
    border: 1px solid var(--color-border-default, #232834);
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
    background-color: var(--color-neutral-200, #E2E8F0);
    transition: transform var(--transition-fast);
    border-radius: 50%;
  }

  input:checked + .toggle-slider {
    background-color: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand, #FF5200);
  }

  input:checked + .toggle-slider:before {
    transform: translateX(16px);
    background-color: var(--color-accent-brand, #FF5200);
  }

  .quick-tone-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid var(--color-border-subtle, #1C2029);
  }

  .tone-slider-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .card-info {
    font-size: 11px;
    color: var(--color-text-secondary, #94A3B8);
    line-height: 1.4;
    margin-bottom: 12px;
  }

  .control-row {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .radio-group {
    display: flex;
    gap: 8px;
  }

  .pill-btn {
    padding: 6px 14px;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 10px;
    font-weight: 800;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .pill-btn:hover {
    color: var(--color-text-primary, #F8FAFC);
    background: var(--color-surface-hover, #262C38);
    border-color: var(--color-border-bright, #2F3646);
  }

  .pill-btn.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand, #FF5200);
    color: var(--color-accent-brand, #FF5200);
  }
</style>
