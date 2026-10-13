<script lang="ts">
  import type { Preset, AudioDevice, PluginCapabilities, ChannelMeter } from '../types';
  import EqGraph from './EqGraph.svelte';
  import { Waves, Shield, Sliders, Volume2, Mic, Search, Star, Headphones, AlertTriangle } from '@lucide/svelte';

  export let currentPreset: Preset;
  export let allPresets: Preset[] = [];
  export let devices: AudioDevice[] = [];
  export let preferredInputDevice: string | null = null;
  export let onChangePreset: (p: Preset) => void;
  export let onOpenBrowser: () => void = () => {};
  export let onSetDevice: (type: 'sink' | 'source', deviceName: string) => void;
  export let micMonitorEnabled: boolean = false;
  export let micMonitorVolume: number = 50;
  export let capabilities: PluginCapabilities | undefined = undefined;
  export let meter: ChannelMeter | undefined = undefined;
  export let onSetMicMonitor: (enabled: boolean, volume: number) => void;

  $: isRnnoiseAvailable = capabilities ? capabilities.rnnoise : true;

  $: noiseAmount = currentPreset.noise_canceller?.amount ?? 0;
  $: gateThreshold = currentPreset.noise_gate?.threshold_db ?? -40;
  $: gateAttack = currentPreset.noise_gate?.attack_ms ?? 10;
  $: gateRelease = currentPreset.noise_gate?.release_ms ?? 100;

  $: compThreshold = currentPreset.compressor?.threshold_db ?? -18;
  $: compRatio = currentPreset.compressor?.ratio ?? 3;
  $: compMakeup = currentPreset.compressor?.makeup_gain_db ?? 2;

  // Real Gain Reduction calculation
  $: compReduction = (() => {
    if (!meter || !(currentPreset.compressor?.enabled ?? true)) return '0.0';
    if (meter.peak_dbfs > compThreshold && compRatio > 1) {
      const red = (meter.peak_dbfs - compThreshold) * (1 - 1 / compRatio);
      return red > 0 ? red.toFixed(1) : '0.0';
    }
    return '0.0';
  })();

  function handleNoiseChange(val: number) {
    if (!currentPreset.noise_canceller) {
      currentPreset.noise_canceller = { enabled: val > 0, amount: val, vad_threshold: 0.5 };
    } else {
      currentPreset.noise_canceller.amount = val;
      currentPreset.noise_canceller.enabled = val > 0;
    }
    onChangePreset(currentPreset);
  }

  function handleGateThresholdChange(val: number) {
    if (!currentPreset.noise_gate) {
      currentPreset.noise_gate = { enabled: true, threshold_db: val, attack_ms: 10, hold_ms: 50, release_ms: 100 };
    } else {
      currentPreset.noise_gate.threshold_db = val;
    }
    onChangePreset(currentPreset);
  }

  function handleGateAttackChange(val: number) {
    if (!currentPreset.noise_gate) {
      currentPreset.noise_gate = { enabled: true, threshold_db: -40, attack_ms: val, hold_ms: 50, release_ms: 100 };
    } else {
      currentPreset.noise_gate.attack_ms = val;
    }
    onChangePreset(currentPreset);
  }

  function handleGateReleaseChange(val: number) {
    if (!currentPreset.noise_gate) {
      currentPreset.noise_gate = { enabled: true, threshold_db: -40, attack_ms: 10, hold_ms: 50, release_ms: val };
    } else {
      currentPreset.noise_gate.release_ms = val;
    }
    onChangePreset(currentPreset);
  }

  function handleCompThresholdChange(val: number) {
    if (!currentPreset.compressor) {
      currentPreset.compressor = { enabled: true, threshold_db: val, ratio: 3, attack_ms: 10, release_ms: 100, makeup_gain_db: 2 };
    } else {
      currentPreset.compressor.threshold_db = val;
    }
    onChangePreset(currentPreset);
  }

  function handleCompRatioChange(val: number) {
    if (!currentPreset.compressor) {
      currentPreset.compressor = { enabled: true, threshold_db: -18, ratio: val, attack_ms: 10, release_ms: 100, makeup_gain_db: 2 };
    } else {
      currentPreset.compressor.ratio = val;
    }
    onChangePreset(currentPreset);
  }

  function handleCompMakeupChange(val: number) {
    if (!currentPreset.compressor) {
      currentPreset.compressor = { enabled: true, threshold_db: -18, ratio: 3, attack_ms: 10, release_ms: 100, makeup_gain_db: val };
    } else {
      currentPreset.compressor.makeup_gain_db = val;
    }
    onChangePreset(currentPreset);
  }

  function handleToneChange(type: 'bass' | 'voice' | 'treble', val: number) {
    if (type === 'bass') currentPreset.eq.bass_db = val;
    if (type === 'voice') currentPreset.eq.voice_db = val;
    if (type === 'treble') currentPreset.eq.treble_db = val;
    onChangePreset(currentPreset);
  }

  $: micPresets = allPresets.filter(p => p.category === 'mic');
</script>

<div class="channel-view mic-view">
  <!-- Preset Bar (1:1 with ChannelTab) -->
  <div class="preset-bar">
    <div class="preset-info">
      <div class="preset-icon-badge">
        <Mic size={18} color="#F97316" />
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

    <div class="header-controls">
      <!-- Input Device Selector -->
      <select
        class="device-select"
        value={preferredInputDevice || 'default'}
        on:change={(e) => onSetDevice('source', e.currentTarget.value)}
      >
        <option value="default">Default Input Device</option>
        {#each (devices || []).filter(d => d.device_type.toLowerCase() === 'source') as dev}
          <option value={dev.name}>{dev.description || dev.name}</option>
        {/each}
      </select>

      <!-- Live Mic Monitor Toggle -->
      <div class="monitor-wrapper" class:active={micMonitorEnabled}>
        <button
          class="monitor-btn"
          class:active={micMonitorEnabled}
          on:click={() => onSetMicMonitor(!micMonitorEnabled, micMonitorVolume)}
          title={micMonitorEnabled ? "Mute live microphone loopback" : "Listen to microphone in real time"}
        >
          <Headphones size={13} />
          <span>{micMonitorEnabled ? 'LIVE MONITORING' : 'MONITOR MIC'}</span>
        </button>
        {#if micMonitorEnabled}
          <div class="monitor-vol-box">
            <input
              type="range"
              min="0"
              max="100"
              value={micMonitorVolume}
              on:input={(e) => onSetMicMonitor(true, Number(e.currentTarget.value))}
              class="monitor-slider"
              title={`Monitor Volume: ${micMonitorVolume}%`}
            />
            <span class="monitor-vol-text">{micMonitorVolume}%</span>
          </div>
        {/if}
      </div>

      <button class="browse-btn" on:click={onOpenBrowser}>
        <Search size={14} />
        <span>BROWSE PRESETS</span>
      </button>
    </div>
  </div>

  <!-- Favorites Strip (1 to 7 slots) -->
  <div class="favorites-strip">
    <span class="fav-label">FAVORITES:</span>
    <div class="fav-slots">
      {#each micPresets.slice(0, 7) as favPreset, idx}
        <button
          class="fav-pill"
          class:active={favPreset.id === currentPreset.id}
          on:click={() => onChangePreset(favPreset)}
        >
          <span class="fav-idx">{idx + 1}</span>
          <span class="fav-title">{favPreset.name}</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Main Content Grid -->
  <div class="cards-grid">
    <!-- Capabilities Banner (if rnnoise missing) -->
    {#if capabilities && !capabilities.rnnoise}
      <div class="caps-banner">
        <AlertTriangle size={15} color="#F59E0B" />
        <span>Noise Remover is unavailable: install librnnoise_ladspa (<code class="caps-pkg">noise-suppression-for-voice</code> from AUR/package manager).</span>
      </div>
    {/if}

    <!-- Equalizer Card -->
    <div class="card eq-card">
      <div class="card-header">
        <div class="card-title">
          <Sliders size={15} color="#F97316" />
          <span>PARAMETRIC EQUALIZER (10-BAND)</span>
        </div>
        <div class="card-actions">
          <label class="toggle-switch">
            <input
              type="checkbox"
              bind:checked={currentPreset.eq.enabled}
              on:change={() => onChangePreset(currentPreset)}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
      </div>

      <!-- Canvas Graph -->
      <EqGraph
        eqConfig={currentPreset.eq}
        accentColor="#F97316"
        onChange={(eq) => {
          currentPreset.eq = eq;
          onChangePreset(currentPreset);
        }}
      />

      <!-- Quick Tone Sliders -->
      <div class="quick-tone-row">
        <div class="tone-slider-group">
          <span class="tone-name">BASS DEPTH</span>
          <input
            type="range"
            min="-12"
            max="12"
            step="0.5"
            value={currentPreset.eq.bass_db ?? 0}
            on:input={(e) => handleToneChange('bass', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val">{(currentPreset.eq.bass_db ?? 0) > 0 ? '+' : ''}{currentPreset.eq.bass_db ?? 0} dB</span>
        </div>
        <div class="tone-slider-group">
          <span class="tone-name">VOCAL CLARITY</span>
          <input
            type="range"
            min="-12"
            max="12"
            step="0.5"
            value={currentPreset.eq.voice_db ?? 0}
            on:input={(e) => handleToneChange('voice', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val">{(currentPreset.eq.voice_db ?? 0) > 0 ? '+' : ''}{currentPreset.eq.voice_db ?? 0} dB</span>
        </div>
        <div class="tone-slider-group">
          <span class="tone-name">CRISP AIR</span>
          <input
            type="range"
            min="-12"
            max="12"
            step="0.5"
            value={currentPreset.eq.treble_db ?? 0}
            on:input={(e) => handleToneChange('treble', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val">{(currentPreset.eq.treble_db ?? 0) > 0 ? '+' : ''}{currentPreset.eq.treble_db ?? 0} dB</span>
        </div>
      </div>
    </div>

    <!-- Bottom DSP Cards Grid -->
    <div class="dsp-grid">
      <!-- Noise Remover (RNNoise) -->
      <div class="card dsp-card" class:is-disabled={!isRnnoiseAvailable}>
        <div class="card-header">
          <div class="card-title">
            <Waves size={15} color="#F97316" />
            <span>NOISE REMOVER</span>
          </div>
          <span class="dsp-badge" class:active={isRnnoiseAvailable && noiseAmount > 0}>
            {#if !isRnnoiseAvailable}
              UNAVAILABLE
            {:else if noiseAmount > 0}
              {noiseAmount}% WET
            {:else}
              BYPASS
            {/if}
          </span>
        </div>
        <p class="dsp-desc">Deep neural network background isolation for loud mechanical keyboards and fans.</p>
        <div class="dsp-presets-row">
          <button class="dsp-pill" class:active={noiseAmount === 0} disabled={!isRnnoiseAvailable} on:click={() => handleNoiseChange(0)}>OFF (0%)</button>
          <button class="dsp-pill" class:active={noiseAmount === 40} disabled={!isRnnoiseAvailable} on:click={() => handleNoiseChange(40)}>LOW (40%)</button>
          <button class="dsp-pill" class:active={noiseAmount === 65} disabled={!isRnnoiseAvailable} on:click={() => handleNoiseChange(65)}>BALANCED (65%)</button>
          <button class="dsp-pill" class:active={noiseAmount === 90} disabled={!isRnnoiseAvailable} on:click={() => handleNoiseChange(90)}>MAX (90%)</button>
        </div>
        <div class="dsp-control-row">
          <span class="dsp-sublabel">Amount</span>
          <input
            type="range"
            min="0"
            max="100"
            value={noiseAmount}
            disabled={!isRnnoiseAvailable}
            on:input={(e) => handleNoiseChange(Number(e.currentTarget.value))}
            class="dsp-slider"
          />
          <span class="dsp-val">{noiseAmount}%</span>
        </div>
      </div>

      <!-- Noise Gate -->
      <div class="card dsp-card">
        <div class="card-header">
          <div class="card-title">
            <Shield size={15} color="#F97316" />
            <span>NOISE GATE</span>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              checked={currentPreset.noise_gate?.enabled ?? true}
              on:change={(e) => {
                if (!currentPreset.noise_gate) {
                  currentPreset.noise_gate = { enabled: e.currentTarget.checked, threshold_db: -40, attack_ms: 10, hold_ms: 50, release_ms: 100 };
                } else {
                  currentPreset.noise_gate.enabled = e.currentTarget.checked;
                }
                onChangePreset(currentPreset);
              }}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
        <p class="dsp-desc">Silences physical microphone when your voice level drops below open threshold.</p>
        <div class="dsp-meter-row">
          {#if meter && meter.peak_dbfs > gateThreshold && (currentPreset.noise_gate?.enabled ?? true)}
            <span class="gate-status-pill open">● GATE OPEN</span>
          {:else}
            <span class="gate-status-pill closed">○ GATE CLOSED</span>
          {/if}
        </div>
        <div class="dsp-controls-list">
          <div class="dsp-control-row">
            <span class="dsp-sublabel">Threshold</span>
            <input
              type="range"
              min="-60"
              max="-10"
              value={gateThreshold}
              on:input={(e) => handleGateThresholdChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val">{gateThreshold} dBFS</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">Attack</span>
            <input
              type="range"
              min="1"
              max="100"
              value={gateAttack}
              on:input={(e) => handleGateAttackChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val">{gateAttack} ms</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">Release</span>
            <input
              type="range"
              min="10"
              max="1000"
              value={gateRelease}
              on:input={(e) => handleGateReleaseChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val">{gateRelease} ms</span>
          </div>
        </div>
      </div>

      <!-- Compressor -->
      <div class="card dsp-card">
        <div class="card-header">
          <div class="card-title">
            <Volume2 size={15} color="#F97316" />
            <span>COMPRESSOR</span>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              checked={currentPreset.compressor?.enabled ?? true}
              on:change={(e) => {
                if (!currentPreset.compressor) {
                  currentPreset.compressor = { enabled: e.currentTarget.checked, threshold_db: -18, ratio: 3, attack_ms: 10, release_ms: 100, makeup_gain_db: 2 };
                } else {
                  currentPreset.compressor.enabled = e.currentTarget.checked;
                }
                onChangePreset(currentPreset);
              }}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
        <p class="dsp-desc">Even out sudden shouting and quiet whispers into broadcast-consistent levels.</p>
        <div class="dsp-meter-row">
          <span class="compressor-gr-pill" class:active={parseFloat(compReduction) > 0}>
            {parseFloat(compReduction) > 0 ? `-${compReduction} dB GAIN REDUCTION` : '0.0 dB GAIN REDUCTION'}
          </span>
        </div>
        <div class="dsp-controls-list">
          <div class="dsp-control-row">
            <span class="dsp-sublabel">Threshold</span>
            <input
              type="range"
              min="-40"
              max="0"
              value={compThreshold}
              on:input={(e) => handleCompThresholdChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val">{compThreshold} dBFS</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">Ratio</span>
            <input
              type="range"
              min="1"
              max="20"
              step="0.5"
              value={compRatio}
              on:input={(e) => handleCompRatioChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val">{compRatio.toFixed(1)}:1</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">Makeup</span>
            <input
              type="range"
              min="0"
              max="24"
              step="0.5"
              value={compMakeup}
              on:input={(e) => handleCompMakeupChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val">{compMakeup > 0 ? '+' : ''}{compMakeup.toFixed(1)} dB</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .channel-view {
    height: 100%;
    overflow-y: auto;
    padding: 24px;
    background: var(--color-bg-app, #0B0E14);
    display: flex;
    flex-direction: column;
    gap: 20px;
    font-family: 'Inter', sans-serif;
  }

  .preset-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--color-surface-1, #11151F);
    padding: 14px 20px;
    border-radius: var(--radius-lg, 8px);
    border: 1px solid var(--color-border-default, #262E40);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  }

  .preset-info {
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .preset-icon-badge {
    width: 38px;
    height: 38px;
    border-radius: var(--radius-md, 6px);
    background: var(--color-surface-3, #1D2333);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid rgba(249, 115, 22, 0.3);
    box-shadow: 0 0 12px rgba(249, 115, 22, 0.15);
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
    color: var(--color-text-primary, #F8FAFC);
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
    color: var(--color-text-secondary, #94A3B8);
  }

  .header-controls {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .device-select {
    background: var(--color-surface-3, #1D2333);
    color: var(--color-text-primary, #F8FAFC);
    border: 1px solid var(--color-border-default, #262E40);
    padding: 7px 12px;
    font-size: 11px;
    font-weight: 500;
    border-radius: var(--radius-sm, 4px);
    outline: none;
    cursor: pointer;
    max-width: 200px;
    text-overflow: ellipsis;
    transition: all 0.15s ease;
  }
  .device-select:hover {
    border-color: var(--color-border-bright, #3B4660);
  }

  .monitor-wrapper {
    display: flex;
    align-items: center;
    background: var(--color-surface-3, #1D2333);
    border: 1px solid var(--color-border-default, #262E40);
    border-radius: var(--radius-sm, 4px);
    transition: all 0.15s ease;
  }
  .monitor-wrapper:hover {
    border-color: var(--color-border-bright, #3B4660);
  }
  .monitor-wrapper.active {
    background: rgba(249, 115, 22, 0.15);
    border-color: rgba(249, 115, 22, 0.6);
    box-shadow: 0 0 10px rgba(249, 115, 22, 0.25);
  }

  .monitor-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 12px;
    background: transparent;
    border: none;
    color: var(--color-text-secondary, #94A3B8);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    transition: color 0.15s ease;
  }
  .monitor-btn:hover {
    color: var(--color-text-primary, #F8FAFC);
  }
  .monitor-btn.active {
    color: #F97316;
  }

  .monitor-vol-box {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-right: 10px;
    padding-left: 2px;
    border-left: 1px solid rgba(249, 115, 22, 0.3);
  }

  .monitor-slider {
    appearance: none;
    width: 60px;
    height: 4px;
    background: var(--color-surface-1, #11151F);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }
  .monitor-slider::-webkit-slider-thumb {
    appearance: none;
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #F97316;
    cursor: pointer;
    box-shadow: 0 0 4px rgba(249, 115, 22, 0.6);
  }

  .monitor-vol-text {
    font-family: var(--font-mono, monospace);
    font-size: 10px;
    font-weight: 700;
    color: #F97316;
    width: 28px;
    text-align: right;
  }

  .browse-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 14px;
    background: var(--color-surface-3, #1D2333);
    border: 1px solid var(--color-border-default, #262E40);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-primary, #F8FAFC);
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .browse-btn:hover {
    background: var(--color-surface-hover, #252D40);
    border-color: var(--color-border-bright, #3B4660);
  }

  .favorites-strip {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .fav-label {
    font-size: 11px;
    font-weight: 800;
    color: var(--color-text-muted, #64748B);
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
    background: var(--color-surface-1, #11151F);
    border: 1px solid var(--color-border-default, #262E40);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 11px;
    cursor: pointer;
    transition: all 0.15s ease;
  }
  .fav-pill:hover {
    color: var(--color-text-primary, #F8FAFC);
    background: var(--color-surface-hover, #252D40);
    border-color: var(--color-border-bright, #3B4660);
  }
  .fav-pill.active {
    background: rgba(249, 115, 22, 0.15);
    border-color: rgba(249, 115, 22, 0.6);
    color: #F97316;
    font-weight: 700;
  }
  .fav-idx {
    font-weight: 800;
    font-family: var(--font-mono, monospace);
    opacity: 0.6;
  }

  .cards-grid {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .card {
    background: var(--color-surface-1, #11151F);
    border: 1px solid var(--color-border-default, #262E40);
    border-radius: var(--radius-lg, 8px);
    padding: 18px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    transition: border-color 0.15s ease;
  }
  .card:hover {
    border-color: var(--color-border-bright, #3B4660);
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
    color: var(--color-text-primary, #F8FAFC);
    letter-spacing: 0.5px;
    text-transform: uppercase;
  }

  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 36px;
    height: 20px;
  }
  .toggle-switch input { opacity: 0; width: 0; height: 0; }
  .toggle-slider {
    position: absolute;
    cursor: pointer;
    top: 0; left: 0; right: 0; bottom: 0;
    background-color: var(--color-surface-3, #1D2333);
    border: 1px solid var(--color-border-default, #262E40);
    transition: .2s;
    border-radius: 20px;
  }
  .toggle-slider:before {
    position: absolute;
    content: "";
    height: 14px;
    width: 14px;
    left: 2px;
    bottom: 2px;
    background-color: var(--color-text-secondary, #94A3B8);
    transition: .2s;
    border-radius: 50%;
  }
  input:checked + .toggle-slider {
    background-color: rgba(249, 115, 22, 0.2);
    border-color: #F97316;
  }
  input:checked + .toggle-slider:before {
    transform: translateX(16px);
    background-color: #F97316;
  }

  .quick-tone-row {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
    margin-top: 16px;
    padding-top: 16px;
    border-top: 1px solid var(--color-border-subtle, #1C2230);
  }

  .tone-slider-group {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .tone-name {
    font-size: 10px;
    font-weight: 800;
    color: var(--color-text-secondary, #94A3B8);
    letter-spacing: 0.5px;
  }

  .tone-slider {
    appearance: none;
    height: 5px;
    background: var(--color-surface-3, #1D2333);
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }
  .tone-slider::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #F97316;
    cursor: pointer;
    box-shadow: 0 0 6px rgba(249, 115, 22, 0.6);
  }

  .tone-val {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-text-primary, #F8FAFC);
  }

  .dsp-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 16px;
  }

  .dsp-card {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
  }

  .dsp-desc {
    font-size: 11px;
    color: var(--color-text-secondary, #94A3B8);
    line-height: 1.4;
    margin-bottom: 14px;
    min-height: 32px;
  }

  .dsp-badge {
    font-size: 9px;
    font-weight: 800;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--color-surface-3, #1D2333);
    color: var(--color-text-muted, #64748B);
    border: 1px solid var(--color-border-default, #262E40);
  }
  .dsp-badge.active {
    background: rgba(249, 115, 22, 0.15);
    color: #F97316;
    border-color: rgba(249, 115, 22, 0.4);
  }

  .dsp-control-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .dsp-sublabel {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-secondary, #94A3B8);
    width: 65px;
  }

  .dsp-slider {
    flex: 1;
    appearance: none;
    height: 5px;
    background: var(--color-surface-3, #1D2333);
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }
  .dsp-slider::-webkit-slider-thumb {
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #F97316;
    cursor: pointer;
    box-shadow: 0 0 6px rgba(249, 115, 22, 0.6);
  }

  .dsp-val {
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-text-primary, #F8FAFC);
    width: 60px;
    text-align: right;
  }

  .dsp-presets-row {
    display: flex;
    gap: 6px;
    margin-bottom: 12px;
  }
  .dsp-pill {
    flex: 1;
    background: var(--color-surface-3, #1D2333);
    border: 1px solid var(--color-border-default, #262E40);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 9px;
    font-weight: 700;
    padding: 5px 4px;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: center;
  }
  .dsp-pill:hover {
    color: var(--color-text-primary, #F8FAFC);
    background: var(--color-surface-hover, #252D40);
    border-color: var(--color-border-bright, #3B4660);
  }
  .dsp-pill.active {
    background: rgba(249, 115, 22, 0.15);
    border-color: #F97316;
    color: #F97316;
  }

  .caps-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    background: rgba(245, 158, 11, 0.08);
    border: 1px solid rgba(245, 158, 11, 0.35);
    border-radius: var(--radius-md, 6px);
    color: #FDE68A;
    font-size: 11px;
    font-weight: 500;
  }
  .caps-pkg {
    font-family: var(--font-mono, monospace);
    background: rgba(0, 0, 0, 0.3);
    padding: 1px 5px;
    border-radius: 3px;
    color: #FCD34D;
    border: 1px solid rgba(245, 158, 11, 0.3);
  }

  .dsp-card.is-disabled {
    opacity: 0.55;
  }
  .dsp-card.is-disabled .dsp-pill,
  .dsp-card.is-disabled .dsp-slider {
    cursor: not-allowed;
  }

  .dsp-controls-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .dsp-meter-row {
    display: flex;
    align-items: center;
    margin-bottom: 12px;
  }
  .gate-status-pill {
    font-size: 10px;
    font-weight: 800;
    padding: 4px 8px;
    border-radius: var(--radius-sm, 4px);
    letter-spacing: 0.5px;
    transition: all 0.15s ease;
  }
  .gate-status-pill.open {
    background: rgba(16, 185, 129, 0.15);
    color: #10B981;
    border: 1px solid rgba(16, 185, 129, 0.4);
    box-shadow: 0 0 8px rgba(16, 185, 129, 0.2);
  }
  .gate-status-pill.closed {
    background: rgba(100, 116, 139, 0.15);
    color: #64748B;
    border: 1px solid rgba(100, 116, 139, 0.3);
  }

  .compressor-gr-pill {
    font-size: 10px;
    font-weight: 800;
    padding: 4px 8px;
    border-radius: var(--radius-sm, 4px);
    background: rgba(100, 116, 139, 0.12);
    color: #64748B;
    border: 1px solid rgba(100, 116, 139, 0.3);
    font-family: var(--font-mono, monospace);
    transition: all 0.15s ease;
  }
  .compressor-gr-pill.active {
    background: rgba(14, 165, 233, 0.18);
    color: #38BDF8;
    border: 1px solid rgba(14, 165, 233, 0.5);
    box-shadow: 0 0 10px rgba(14, 165, 233, 0.25);
  }
</style>
