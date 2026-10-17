<script lang="ts">
  import type { ChannelId, Preset, ChannelMeter } from '../types';
  import { CHANNELS } from './theme';
  import EqGraph from './EqGraph.svelte';
  import SonarSlider from './SonarSlider.svelte';
  import { Search, Star, Sliders, Waves, Sparkles, Volume2, BookmarkPlus, Check, X } from '@lucide/svelte';
  import { t } from './i18n';

  export let channel: ChannelId;
  export let currentPreset: Preset;
  export let allPresets: Preset[];
  export let favoritePresets: string[] = [];
  export let meter: ChannelMeter | undefined = undefined;
  export let onApplyPreset: (preset: Preset) => void;
  export let onOpenBrowser: () => void;
  export let onChangeEq: (eq: Preset['eq']) => void;
  export let onToggleFavorite: ((presetId: string) => void) | undefined = undefined;
  export let onSaveCustomPreset: ((channel: ChannelId, preset: Preset) => void) | undefined = undefined;

  const info = CHANNELS[channel];

  let isSavingCustom = false;
  let customPresetName = '';

  $: if (!currentPreset.spatial) {
    currentPreset.spatial = { enabled: false, immersion_pct: 50 };
  }
  $: if (!currentPreset.smart_volume) {
    currentPreset.smart_volume = { enabled: false, mode: 'balanced' };
  }

  function handleToneChange(type: 'bass' | 'voice' | 'treble', val: number) {
    if (type === 'bass') currentPreset.eq.bass_db = val;
    if (type === 'voice') currentPreset.eq.voice_db = val;
    if (type === 'treble') currentPreset.eq.treble_db = val;
    onChangeEq(currentPreset.eq);
    onApplyPreset(currentPreset);
  }

  function handleToggleBypass() {
    currentPreset.eq.enabled = !currentPreset.eq.enabled;
    onChangeEq(currentPreset.eq);
    onApplyPreset(currentPreset);
  }

  function openSaveCustomModal() {
    customPresetName = currentPreset.name + ' (Custom)';
    isSavingCustom = true;
  }

  function handleConfirmSaveCustom() {
    const trimmed = customPresetName.trim();
    if (!trimmed) return;
    const newPreset: Preset = {
      ...JSON.parse(JSON.stringify(currentPreset)),
      id: `custom_${Date.now()}`,
      name: trimmed,
      description: `Custom tuned preset for ${channel}`,
      category: 'custom',
    };
    if (onSaveCustomPreset) {
      onSaveCustomPreset(channel, newPreset);
    }
    isSavingCustom = false;
  }

  // Filter presets for this channel's category
  $: categoryPresets = allPresets.filter(p => p.category === channel || p.category === 'custom');
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
          <span class="preset-name">{currentPreset.name}</span>
          <button
            class="star-btn"
            class:is-fav={favoritePresets.includes(currentPreset.id)}
            aria-label={$t('channel_tab.add_favorite', { name: currentPreset.name })}
            title={favoritePresets.includes(currentPreset.id) ? 'Remove Favorite' : 'Add Favorite'}
            on:click={() => onToggleFavorite?.(currentPreset.id)}
          >
            <Star
              size={15}
              fill={favoritePresets.includes(currentPreset.id) ? '#eab308' : 'none'}
              color={favoritePresets.includes(currentPreset.id) ? '#eab308' : '#64748B'}
            />
          </button>
        </div>
        <span class="preset-desc">{currentPreset.description}</span>
      </div>
    </div>

    <div class="preset-actions-right">
      <button class="save-custom-btn" on:click={openSaveCustomModal} title="Save current settings as new custom preset">
        <BookmarkPlus size={14} />
        <span>Save Custom</span>
      </button>

      <button class="browse-btn" aria-label={$t('channel_tab.browse_presets_aria', { channel: $t(`channels.${channel}`) })} on:click={onOpenBrowser}>
        <Search size={14} />
        <span>{$t('channel_tab.browse_presets')}</span>
      </button>
    </div>
  </div>

  <!-- Favorites Strip (1 to 7 slots) -->
  <div class="favorites-strip">
    <span class="fav-label">{$t('channel_tab.favorites')}</span>
    <div class="fav-slots">
      {#each categoryPresets.slice(0, 7) as favPreset, idx}
        <button
          class="fav-pill"
          class:active={favPreset.id === currentPreset.id}
          style={favPreset.id === currentPreset.id ? `border-color: ${info.defaultColor}; color: ${info.defaultColor}; background: ${info.defaultColor}18;` : ''}
          aria-label={$t('channel_tab.select_favorite', { idx: idx + 1, name: favPreset.name })}
          on:click={() => onApplyPreset(favPreset)}
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
            class="bypass-btn"
            class:bypassed={!currentPreset.eq.enabled}
            on:click={handleToggleBypass}
            title={currentPreset.eq.enabled ? $t('channel_tab.bypass_eq') : $t('channel_tab.enable_eq')}
            aria-label={currentPreset.eq.enabled ? $t('channel_tab.bypass_eq') : $t('channel_tab.enable_eq')}
          >
            {!currentPreset.eq.enabled ? $t('channel_tab.eq_bypassed') : $t('channel_tab.bypass_eq')}
          </button>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('channel_tab.toggle_eq_aria')}
              bind:checked={currentPreset.eq.enabled}
              on:change={() => {
                onChangeEq(currentPreset.eq);
                onApplyPreset(currentPreset);
              }}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
      </div>

      <!-- Canvas Graph with live RTA spectrum & 10-band inspector table -->
      <EqGraph
        eqConfig={currentPreset.eq}
        {meter}
        accentColor={info.defaultColor}
        onChange={(eq) => {
          currentPreset.eq = eq;
          onChangeEq(eq);
          onApplyPreset(currentPreset);
        }}
      />

      <!-- Quick Tone Sliders -->
      <div class="quick-tone-row">
        <div class="tone-slider-group">
          <SonarSlider
            label={$t('channel_tab.bass_depth')}
            value={currentPreset.eq.bass_db}
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
            value={currentPreset.eq.voice_db}
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
            value={currentPreset.eq.treble_db}
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
    {#if channel === 'game' && currentPreset.spatial}
      <div class="card">
        <div class="card-header">
          <div class="card-title">
            <Sparkles size={15} color="var(--color-channel-game)" />
            <span>{$t('channel_tab.spatial_audio')}</span>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('channel_tab.toggle_spatial_aria')}
              bind:checked={currentPreset.spatial.enabled}
              on:change={() => onApplyPreset(currentPreset)}
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
            value={currentPreset.spatial.immersion_pct}
            min={0}
            max={100}
            step={1}
            unit="%"
            accentColor="var(--color-channel-game)"
            disabled={!currentPreset.spatial.enabled}
            onChange={(v) => {
              if (currentPreset.spatial) {
                currentPreset.spatial.immersion_pct = v;
                onApplyPreset(currentPreset);
              }
            }}
          />
        </div>
      </div>

      {#if currentPreset.smart_volume}
        <div class="card">
          <div class="card-header">
            <div class="card-title">
              <Volume2 size={15} color="var(--color-channel-game)" />
              <span>{$t('channel_tab.volume_normalizer')}</span>
            </div>
            <label class="toggle-switch">
              <input
                type="checkbox"
                bind:checked={currentPreset.smart_volume.enabled}
                on:change={() => onApplyPreset(currentPreset)}
              />
              <span class="toggle-slider"></span>
            </label>
          </div>
          <p class="card-info">
            {$t('channel_tab.volume_normalizer_desc')}
          </p>
          <div class="radio-group" class:disabled={!currentPreset.smart_volume.enabled}>
            <button
              class="pill-btn"
              class:active={currentPreset.smart_volume.mode === 'quiet'}
              on:click={() => {
                if (currentPreset.smart_volume) {
                  currentPreset.smart_volume.mode = 'quiet';
                  currentPreset.smart_volume.enabled = true;
                  onApplyPreset(currentPreset);
                }
              }}
            >{$t('channel_tab.quiet')}</button>
            <button
              class="pill-btn"
              class:active={currentPreset.smart_volume.mode === 'balanced'}
              on:click={() => {
                if (currentPreset.smart_volume) {
                  currentPreset.smart_volume.mode = 'balanced';
                  currentPreset.smart_volume.enabled = true;
                  onApplyPreset(currentPreset);
                }
              }}
            >{$t('channel_tab.balanced')}</button>
            <button
              class="pill-btn"
              class:active={currentPreset.smart_volume.mode === 'loud'}
              on:click={() => {
                if (currentPreset.smart_volume) {
                  currentPreset.smart_volume.mode = 'loud';
                  currentPreset.smart_volume.enabled = true;
                  onApplyPreset(currentPreset);
                }
              }}
            >{$t('channel_tab.loud')}</button>
          </div>
        </div>
      {/if}
    {/if}

    {#if channel === 'chat'}
      <div class="card">
        <div class="card-header">
          <div class="card-title">
            <Waves size={15} color="var(--color-channel-chat, #22C55E)" />
            <span>{$t('channel_tab.noise_remover')}</span>
          </div>
          <button
            class="toggle-btn"
            class:active={currentPreset.noise_canceller?.enabled ?? false}
            on:click={() => {
              if (!currentPreset.noise_canceller) {
                currentPreset.noise_canceller = { enabled: true, amount: 65, vad_threshold: 0.35 };
              } else {
                currentPreset.noise_canceller.enabled = !currentPreset.noise_canceller.enabled;
              }
              onApplyPreset(currentPreset);
            }}
          >
            {currentPreset.noise_canceller?.enabled ? $t('common.on') : $t('common.off')}
          </button>
        </div>
        <p class="card-info">
          {$t('channel_tab.noise_remover_desc')}
        </p>
        <div class="control-row">
          <SonarSlider
            label={$t('channel_tab.suppression')}
            min={0}
            max={100}
            step={1}
            unit="%"
            accentColor="var(--color-channel-chat, #22C55E)"
            value={currentPreset.noise_canceller?.amount ?? 0}
            disabled={!currentPreset.noise_canceller?.enabled}
            onChange={(val) => {
              if (!currentPreset.noise_canceller) {
                currentPreset.noise_canceller = { enabled: true, amount: val, vad_threshold: 0.35 };
              } else {
                currentPreset.noise_canceller.amount = val;
              }
              onApplyPreset(currentPreset);
            }}
          />
        </div>
      </div>
    {/if}
  </div>

  {#if isSavingCustom}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="custom-modal-backdrop" on:click={() => isSavingCustom = false}>
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
          <button type="button" class="btn-cancel" on:click={() => isSavingCustom = false}>Cancel</button>
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
    padding: 24px;
    background: var(--color-bg-app);
    display: flex;
    flex-direction: column;
    gap: 16px;
    user-select: none;
    font-family: var(--font-sans);
  }

  .preset-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--color-surface-1);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-lg);
    padding: 14px 20px;
    box-shadow: var(--shadow-depth-sm);
  }

  .preset-info {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .preset-icon-badge {
    width: 38px;
    height: 38px;
    border-radius: var(--radius-md);
    background: var(--color-surface-3);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--color-border-default);
    box-shadow: var(--shadow-depth-sm);
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
    font-size: 15px;
    font-weight: 700;
    color: var(--color-text-primary);
  }

  .star-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
  }

  .preset-desc {
    font-size: 12px;
    color: var(--color-text-secondary);
  }

  .browse-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 14px;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    color: var(--color-text-primary);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: background-color var(--transition-fast), border-color var(--transition-fast);
  }
  .preset-actions-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .save-custom-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: var(--color-surface-2, #161B26);
    border: 1px solid var(--color-border-bright, #2A344A);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-neutral-300, #CBD5E1);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    transition: all var(--transition-fast, 120ms);
  }

  .save-custom-btn:hover {
    background: var(--color-surface-hover, #252D40);
    color: var(--color-accent-brand, #FF5200);
    border-color: var(--color-accent-brand, #FF5200);
  }

  .custom-modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    backdrop-filter: blur(4px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .custom-modal-box {
    background: var(--color-surface-1, #11151F);
    border: 1px solid var(--color-border-bright, #2A344A);
    border-radius: var(--radius-lg, 8px);
    box-shadow: var(--shadow-depth-lg, 0 6px 20px rgba(0,0,0,0.55));
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
    background: var(--color-surface-2, #161B26);
    border: 1px solid var(--color-border-default, #1F2637);
    border-radius: var(--radius-sm, 4px);
    padding: 8px 12px;
    color: var(--color-text-primary, #F8FAFC);
    font-size: 0.85rem;
    font-family: var(--font-sans);
    outline: none;
    transition: border-color var(--transition-fast, 120ms);
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
    border: 1px solid var(--color-border-default, #1F2637);
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
    transition: opacity var(--transition-fast, 120ms);
  }

  .btn-confirm:hover {
    opacity: 0.9;
  }

  .favorites-strip {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .fav-label {
    font-size: 11px;
    font-weight: 800;
    color: var(--color-text-muted);
    letter-spacing: 0.5px;
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
    padding: 5px 12px;
    background: var(--color-surface-1);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    font-size: 11px;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: color var(--transition-fast), background-color var(--transition-fast), border-color var(--transition-fast);
  }
  .fav-pill:hover {
    color: var(--color-text-primary);
    background: var(--color-surface-hover);
    border-color: var(--color-border-bright);
  }

  .fav-idx {
    font-weight: 800;
    font-family: var(--font-mono);
    opacity: 0.6;
  }

  .cards-grid {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card {
    background: var(--color-surface-1);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-lg);
    padding: 18px;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .card:hover {
    border-color: var(--color-border-bright);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }

  .card-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    font-weight: 800;
    color: var(--color-text-primary);
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .bypass-btn {
    padding: 4px 10px;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    font-size: 10px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    letter-spacing: 0.5px;
    transition: color var(--transition-fast), border-color var(--transition-fast), background-color var(--transition-fast);
  }
  .bypass-btn:hover {
    color: var(--color-text-primary);
    border-color: var(--color-border-bright);
    background: var(--color-surface-hover);
  }
  .bypass-btn.bypassed {
    background: var(--color-accent-danger-subtle);
    border-color: var(--color-accent-danger);
    color: var(--color-accent-danger);
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

  .quick-tone-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--color-border-subtle);
  }

  .tone-slider-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .card-info {
    font-size: 11px;
    color: var(--color-text-secondary);
    line-height: 1.4;
    margin-bottom: 14px;
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
  .pill-btn:hover {
    color: var(--color-text-primary);
    background: var(--color-surface-hover);
    border-color: var(--color-border-bright);
  }
  .pill-btn.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand);
    color: var(--color-accent-brand);
  }
</style>
