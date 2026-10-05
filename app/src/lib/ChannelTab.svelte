<script lang="ts">
  import type { ChannelId, Preset } from '../types';
  import { CHANNELS } from './theme';
  import EqGraph from './EqGraph.svelte';
  import { Search, Star, Sliders, Waves, Sparkles, Volume2, Shield } from '@lucide/svelte';

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
  }

  // Filter presets for this channel's category
  $: categoryPresets = allPresets.filter(p => p.category === channel);
</script>

<div class="channel-view">
  <!-- Preset Bar -->
  <div class="preset-bar">
    <div class="preset-info">
      <div class="preset-icon-badge" style="border-color: {info.defaultColor}44;">
        <Sliders size={18} color={info.defaultColor} />
      </div>
      <div class="preset-titles">
        <div class="preset-title-row">
          <span class="preset-name">{currentPreset.name}</span>
          <button class="star-btn" title="Add to Favorites">
            <Star size={14} fill="#eab308" color="#eab308" />
          </button>
        </div>
        <span class="preset-desc">{currentPreset.description}</span>
      </div>
    </div>

    <button class="browse-btn" on:click={onOpenBrowser}>
      <Search size={14} />
      <span>BROWSE PRESETS</span>
    </button>
  </div>

  <!-- Favorites Strip (1 to 9 slots) -->
  <div class="favorites-strip">
    <span class="fav-label">FAVORITES:</span>
    <div class="fav-slots">
      {#each categoryPresets.slice(0, 7) as favPreset, idx}
        <button
          class="fav-pill"
          class:active={favPreset.id === currentPreset.id}
          on:click={() => onApplyPreset(favPreset)}
        >
          <span class="fav-idx">{idx + 1}</span>
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
          <label class="toggle-switch">
            <input
              type="checkbox"
              bind:checked={currentPreset.eq.enabled}
              on:change={() => onChangeEq(currentPreset.eq)}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
      </div>

      <!-- Canvas Graph -->
      <EqGraph
        eqConfig={currentPreset.eq}
        accentColor={info.defaultColor}
        onChange={onChangeEq}
      />

      <!-- Quick Tone Sliders -->
      <div class="quick-tone-row">
        <div class="tone-slider-group">
          <span class="tone-name">BASS</span>
          <input
            type="range"
            min="-12"
            max="12"
            step="0.5"
            bind:value={currentPreset.eq.bass_db}
            on:input={(e) => handleToneChange('bass', Number(e.currentTarget.value))}
          />
          <span class="tone-val">{currentPreset.eq.bass_db > 0 ? '+' : ''}{currentPreset.eq.bass_db} dB</span>
        </div>

        <div class="tone-slider-group">
          <span class="tone-name">VOICE</span>
          <input
            type="range"
            min="-12"
            max="12"
            step="0.5"
            bind:value={currentPreset.eq.voice_db}
            on:input={(e) => handleToneChange('voice', Number(e.currentTarget.value))}
          />
          <span class="tone-val">{currentPreset.eq.voice_db > 0 ? '+' : ''}{currentPreset.eq.voice_db} dB</span>
        </div>

        <div class="tone-slider-group">
          <span class="tone-name">TREBLE</span>
          <input
            type="range"
            min="-12"
            max="12"
            step="0.5"
            bind:value={currentPreset.eq.treble_db}
            on:input={(e) => handleToneChange('treble', Number(e.currentTarget.value))}
          />
          <span class="tone-val">{currentPreset.eq.treble_db > 0 ? '+' : ''}{currentPreset.eq.treble_db} dB</span>
        </div>
      </div>
    </div>

    <!-- Channel Specific Cards -->
    {#if channel === 'game'}
      <div class="card">
        <div class="card-header">
          <div class="card-title">
            <Sparkles size={15} color="#22c55e" />
            <span>SPATIAL AUDIO (VIRTUAL 7.1 SURROUND)</span>
          </div>
          <label class="toggle-switch">
            <input type="checkbox" bind:checked={spatialEnabled} />
            <span class="toggle-slider"></span>
          </label>
        </div>
        <p class="card-info">
          HRTF directional soundstage virtualization for pin-point enemy footstep cues.
        </p>
        <div class="control-row">
          <span class="control-label">Immersion Distance</span>
          <input type="range" min="0" max="100" bind:value={spatialDistance} class="effect-slider" />
          <span class="control-val">{spatialDistance}%</span>
        </div>
      </div>

      <div class="card">
        <div class="card-header">
          <div class="card-title">
            <Volume2 size={15} color="#22c55e" />
            <span>SMART VOLUME NORMALIZATION</span>
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
            <Waves size={15} color="#06b6d4" />
            <span>CLEARCAST AI NOISE CANCELLATION</span>
          </div>
        </div>
        <p class="card-info">
          Suppresses incoming background static, keyboard clicks, and room noise from friends' mics.
        </p>
        <div class="control-row">
          <span class="control-label">Suppression</span>
          <input type="range" min="0" max="100" bind:value={chatNoiseAmount} class="effect-slider" />
          <span class="control-val">{chatNoiseAmount > 0 ? chatNoiseAmount + '%' : 'OFF'}</span>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .channel-view {
    height: calc(100vh - 52px);
    overflow-y: auto;
    padding: 16px 24px;
    background: #0d1117;
    display: flex;
    flex-direction: column;
    gap: 16px;
    user-select: none;
  }

  .preset-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #161b22;
    border: 1px solid #232b36;
    border-radius: 8px;
    padding: 12px 18px;
  }

  .preset-info {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .preset-icon-badge {
    width: 38px;
    height: 38px;
    border-radius: 8px;
    background: #1c2430;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid #2d3846;
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
    color: #f1f5f9;
  }

  .star-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0;
  }

  .preset-desc {
    font-size: 12px;
    color: #94a3b8;
  }

  .browse-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 14px;
    background: #202733;
    border: 1px solid #2e3846;
    border-radius: 6px;
    color: #e2e8f0;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .browse-btn:hover {
    background: #2a3444;
    border-color: #3e4c60;
  }

  .favorites-strip {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .fav-label {
    font-size: 11px;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.5px;
  }

  .fav-slots {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .fav-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    background: #161b22;
    border: 1px solid #242c38;
    border-radius: 6px;
    color: #94a3b8;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .fav-pill:hover {
    color: #f1f5f9;
    background: #202834;
  }

  .fav-pill.active {
    background: #22c55e1a;
    border-color: #22c55e66;
    color: #4ade80;
    font-weight: 600;
  }

  .fav-idx {
    font-weight: 700;
    font-family: monospace;
    opacity: 0.6;
  }

  .cards-grid {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card {
    background: #161b22;
    border: 1px solid #232b36;
    border-radius: 8px;
    padding: 16px;
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
    font-size: 13px;
    font-weight: 700;
    color: #e2e8f0;
    letter-spacing: 0.5px;
  }

  .card-info {
    font-size: 12px;
    color: #94a3b8;
    margin-bottom: 12px;
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
    background-color: #22c55e;
  }

  input:checked + .toggle-slider:before {
    transform: translateX(16px);
  }

  .quick-tone-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    margin-top: 14px;
    padding-top: 14px;
    border-top: 1px solid #232b36;
  }

  .tone-slider-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .tone-name {
    font-size: 11px;
    font-weight: 700;
    color: #94a3b8;
  }

  .tone-val {
    font-family: monospace;
    font-size: 11px;
    color: #cbd5e1;
  }

  .control-row {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .control-label {
    width: 130px;
    font-size: 12px;
    font-weight: 600;
    color: #94a3b8;
  }

  .effect-slider {
    flex: 1;
    appearance: none;
    height: 6px;
    background: #242c38;
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }

  .effect-slider::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #f1f5f9;
    cursor: pointer;
  }

  .control-val {
    width: 45px;
    font-family: monospace;
    font-size: 12px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .radio-group {
    display: flex;
    gap: 8px;
  }

  .pill-btn {
    padding: 6px 14px;
    background: #1e2530;
    border: 1px solid #2d3846;
    border-radius: 6px;
    color: #94a3b8;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
  }

  .pill-btn.active {
    background: #22c55e22;
    border-color: #22c55e;
    color: #4ade80;
  }
</style>
