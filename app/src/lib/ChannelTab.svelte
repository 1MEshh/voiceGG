<script lang="ts">
  import type { ChannelId, Preset } from '../types';
  import { CHANNELS } from './theme';
  import EqGraph from './EqGraph.svelte';
  import { Search, Star, Sliders, Waves, Sparkles, Volume2 } from '@lucide/svelte';

  export let channel: ChannelId;
  export let currentPreset: Preset;
  export let allPresets: Preset[];
  export let onApplyPreset: (preset: Preset) => void;
  export let onOpenBrowser: () => void;
  export let onChangeEq: (eq: Preset['eq']) => void;

  const info = CHANNELS[channel];

  let spatialEnabled = false;
  let spatialDistance = 50;
  let smartVolume = 'balanced';
  let chatNoiseAmount = currentPreset.noise_canceller?.amount ?? 60;

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

  // Filter presets for this channel's category
  $: categoryPresets = allPresets.filter(p => p.category === channel);
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
          <button class="star-btn" aria-label="Add {currentPreset.name} to Favorites" title="Add to Favorites">
            <Star size={14} fill="#eab308" color="#eab308" />
          </button>
        </div>
        <span class="preset-desc">{currentPreset.description}</span>
      </div>
    </div>

    <button class="browse-btn" aria-label="Browse {info.label} presets" on:click={onOpenBrowser}>
      <Search size={14} />
      <span>BROWSE PRESETS</span>
    </button>
  </div>

  <!-- Favorites Strip (1 to 7 slots) -->
  <div class="favorites-strip">
    <span class="fav-label">FAVORITES:</span>
    <div class="fav-slots">
      {#each categoryPresets.slice(0, 7) as favPreset, idx}
        <button
          class="fav-pill"
          class:active={favPreset.id === currentPreset.id}
          style={favPreset.id === currentPreset.id ? `border-color: ${info.defaultColor}; color: ${info.defaultColor}; background: ${info.defaultColor}18;` : ''}
          aria-label="Select favorite {idx + 1}: {favPreset.name}"
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
          <span>PARAMETRIC EQUALIZER (10-BAND)</span>
        </div>
        <div class="card-actions">
          <button
            class="bypass-btn"
            class:bypassed={!currentPreset.eq.enabled}
            on:click={handleToggleBypass}
            title={currentPreset.eq.enabled ? "Bypass Equalizer" : "Enable Equalizer"}
            aria-label={currentPreset.eq.enabled ? "Bypass Equalizer" : "Enable Equalizer"}
          >
            {!currentPreset.eq.enabled ? 'EQ BYPASSED' : 'BYPASS EQ'}
          </button>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label="Toggle Equalizer Enabled"
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

      <!-- Canvas Graph -->
      <EqGraph
        eqConfig={currentPreset.eq}
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
          <span class="tone-name">BASS</span>
          <input
            type="range"
            aria-label="Bass Tone Gain"
            aria-valuenow={currentPreset.eq.bass_db}
            aria-valuemin="-12"
            aria-valuemax="12"
            min="-12"
            max="12"
            step="0.5"
            bind:value={currentPreset.eq.bass_db}
            on:input={(e) => handleToneChange('bass', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val tabular-nums">{currentPreset.eq.bass_db > 0 ? '+' : ''}{currentPreset.eq.bass_db} dB</span>
        </div>

        <div class="tone-slider-group">
          <span class="tone-name">VOICE</span>
          <input
            type="range"
            aria-label="Vocal Tone Gain"
            aria-valuenow={currentPreset.eq.voice_db}
            aria-valuemin="-12"
            aria-valuemax="12"
            min="-12"
            max="12"
            step="0.5"
            bind:value={currentPreset.eq.voice_db}
            on:input={(e) => handleToneChange('voice', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val tabular-nums">{currentPreset.eq.voice_db > 0 ? '+' : ''}{currentPreset.eq.voice_db} dB</span>
        </div>

        <div class="tone-slider-group">
          <span class="tone-name">TREBLE</span>
          <input
            type="range"
            aria-label="Treble Tone Gain"
            aria-valuenow={currentPreset.eq.treble_db}
            aria-valuemin="-12"
            aria-valuemax="12"
            min="-12"
            max="12"
            step="0.5"
            bind:value={currentPreset.eq.treble_db}
            on:input={(e) => handleToneChange('treble', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val tabular-nums">{currentPreset.eq.treble_db > 0 ? '+' : ''}{currentPreset.eq.treble_db} dB</span>
        </div>
      </div>
    </div>

    <!-- Channel Specific Cards -->
    {#if channel === 'game'}
      <div class="card">
        <div class="card-header">
          <div class="card-title">
            <Sparkles size={15} color="var(--color-channel-game)" />
            <span>SPATIAL AUDIO (VIRTUAL SURROUND)</span>
          </div>
          <label class="toggle-switch">
            <input type="checkbox" aria-label="Toggle Spatial Audio" bind:checked={spatialEnabled} />
            <span class="toggle-slider"></span>
          </label>
        </div>
        <p class="card-info">
          Directional soundstage virtualization for pin-point enemy footstep cues.
        </p>
        <div class="control-row">
          <span class="control-label">Immersion Distance</span>
          <input
            type="range"
            aria-label="Immersion Distance"
            aria-valuenow={spatialDistance}
            aria-valuemin="0"
            aria-valuemax="100"
            min="0"
            max="100"
            bind:value={spatialDistance}
            class="effect-slider"
          />
          <span class="control-val tabular-nums">{spatialDistance}%</span>
        </div>
      </div>

      <div class="card">
        <div class="card-header">
          <div class="card-title">
            <Volume2 size={15} color="var(--color-channel-game)" />
            <span>VOLUME NORMALIZER</span>
          </div>
        </div>
        <p class="card-info">
          Clamps explosive peaks while amplifying quiet ambient footsteps.
        </p>
        <div class="radio-group">
          <button class="pill-btn" class:active={smartVolume === 'quiet'} on:click={() => smartVolume = 'quiet'}>QUIET</button>
          <button class="pill-btn" class:active={smartVolume === 'balanced'} on:click={() => smartVolume = 'balanced'}>BALANCED</button>
          <button class="pill-btn" class:active={smartVolume === 'loud'} on:click={() => smartVolume = 'loud'}>LOUD</button>
        </div>
      </div>
    {/if}

    {#if channel === 'chat'}
      <div class="card">
        <div class="card-header">
          <div class="card-title">
            <Waves size={15} color="var(--color-channel-chat)" />
            <span>NOISE REMOVER</span>
          </div>
        </div>
        <p class="card-info">
          Suppresses incoming background static, keyboard clicks, and room noise from incoming voice chat.
        </p>
        <div class="control-row">
          <span class="control-label">Suppression</span>
          <input
            type="range"
            aria-label="Noise Suppression"
            aria-valuenow={chatNoiseAmount}
            aria-valuemin="0"
            aria-valuemax="100"
            min="0"
            max="100"
            bind:value={chatNoiseAmount}
            class="effect-slider"
          />
          <span class="control-val tabular-nums">{chatNoiseAmount > 0 ? chatNoiseAmount + '%' : 'OFF'}</span>
        </div>
      </div>
    {/if}
  </div>
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
  .browse-btn:hover {
    background: var(--color-surface-hover);
    border-color: var(--color-border-bright);
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

  .tone-name {
    font-size: 10px;
    font-weight: 800;
    color: var(--color-text-secondary);
    letter-spacing: 0.5px;
  }

  .tone-slider {
    appearance: none;
    -webkit-appearance: none;
    height: 5px;
    background: var(--color-surface-3);
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }
  .tone-slider::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border-radius: var(--radius-sm);
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    cursor: pointer;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .tone-slider:hover::-webkit-slider-thumb {
    border-color: var(--color-accent-brand);
  }
  .tone-slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: var(--radius-sm);
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    cursor: pointer;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .tone-slider:hover::-moz-range-thumb {
    border-color: var(--color-accent-brand);
  }

  .tone-val {
    font-family: var(--font-mono);
    font-variant-numeric: var(--tabular-nums);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-text-primary);
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

  .control-label {
    width: 130px;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-secondary);
  }

  .effect-slider {
    flex: 1;
    appearance: none;
    -webkit-appearance: none;
    height: 5px;
    background: var(--color-surface-3);
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }
  .effect-slider::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border-radius: var(--radius-sm);
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    cursor: pointer;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .effect-slider:hover::-webkit-slider-thumb {
    border-color: var(--color-accent-brand);
  }
  .effect-slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: var(--radius-sm);
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    cursor: pointer;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .effect-slider:hover::-moz-range-thumb {
    border-color: var(--color-accent-brand);
  }

  .control-val {
    width: 45px;
    font-family: var(--font-mono);
    font-variant-numeric: var(--tabular-nums);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-text-primary);
    text-align: right;
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
