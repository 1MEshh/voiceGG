<script lang="ts">
  import type { Preset, AudioDevice, PluginCapabilities, ChannelMeter } from '../types';
  import EqGraph from './EqGraph.svelte';
  import { Waves, Shield, Sliders, Volume2, Mic, Search, Star, Headphones, AlertTriangle } from '@lucide/svelte';
  import { t } from './i18n';

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
  $: gateHysteresis = currentPreset.noise_gate?.hysteresis_db ?? 6;

  $: compThreshold = currentPreset.compressor?.threshold_db ?? -18;
  $: compRatio = currentPreset.compressor?.ratio ?? 3;
  $: compMakeup = currentPreset.compressor?.makeup_gain_db ?? 2;
  $: compKnee = currentPreset.compressor?.knee_db ?? 3;

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

  function handleGateHysteresisChange(val: number) {
    if (!currentPreset.noise_gate) {
      currentPreset.noise_gate = { enabled: true, threshold_db: -40, attack_ms: 10, hold_ms: 50, release_ms: 100, hysteresis_db: val };
    } else {
      currentPreset.noise_gate.hysteresis_db = val;
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

  function handleCompKneeChange(val: number) {
    if (!currentPreset.compressor) {
      currentPreset.compressor = { enabled: true, threshold_db: -18, ratio: 3, attack_ms: 10, release_ms: 100, makeup_gain_db: 2, knee_db: val };
    } else {
      currentPreset.compressor.knee_db = val;
    }
    onChangePreset(currentPreset);
  }

  function handleToneChange(type: 'bass' | 'voice' | 'treble', val: number) {
    if (type === 'bass') currentPreset.eq.bass_db = val;
    if (type === 'voice') currentPreset.eq.voice_db = val;
    if (type === 'treble') currentPreset.eq.treble_db = val;
    onChangePreset(currentPreset);
  }

  let micInputGain = 0;
  function handleInputGainChange(val: number) {
    micInputGain = val;
    onChangePreset(currentPreset);
  }

  $: micPresets = allPresets.filter(p => p.category === 'mic');
</script>

<div class="channel-view mic-view">
  <!-- Preset Bar (1:1 with ChannelTab) -->
  <div class="preset-bar">
    <div class="preset-info">
      <div class="preset-icon-badge">
        <Mic size={18} color="var(--color-channel-mic)" />
      </div>
      <div class="preset-titles">
        <div class="preset-title-row">
          <span class="preset-name">{currentPreset.name}</span>
          <button class="star-btn" aria-label={$t('mic.add_favorite', { name: currentPreset.name })} title={$t('mic.add_favorite', { name: currentPreset.name })}>
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
        aria-label={$t('mic.input_selector_aria')}
        value={preferredInputDevice || 'default'}
        on:change={(e) => onSetDevice('source', e.currentTarget.value)}
      >
        <option value="default">{$t('mic.default_input')}</option>
        {#each (devices || []).filter(d => d.device_type.toLowerCase() === 'source') as dev}
          <option value={dev.name}>{dev.description || dev.name}</option>
        {/each}
      </select>

      <!-- Live Mic Monitor Toggle -->
      <div class="monitor-wrapper" class:active={micMonitorEnabled}>
        <button
          class="monitor-btn"
          class:active={micMonitorEnabled}
          aria-label={micMonitorEnabled ? $t('mic.mute_loopback') : $t('mic.listen_realtime')}
          on:click={() => onSetMicMonitor(!micMonitorEnabled, micMonitorVolume)}
          title={micMonitorEnabled ? $t('mic.mute_loopback') : $t('mic.listen_realtime')}
        >
          <Headphones size={13} />
          <span>{micMonitorEnabled ? $t('mic.live_monitoring_btn') : $t('mic.monitor_mic')}</span>
        </button>
        {#if micMonitorEnabled}
          <div class="monitor-vol-box">
            <input
              type="range"
              aria-label={$t('mic.monitor_vol_aria')}
              aria-valuenow={micMonitorVolume}
              aria-valuemin="0"
              aria-valuemax="100"
              min="0"
              max="100"
              value={micMonitorVolume}
              on:input={(e) => onSetMicMonitor(true, Number(e.currentTarget.value))}
              class="monitor-slider"
              title={$t('mic.monitor_vol_title', { vol: micMonitorVolume })}
            />
            <span class="monitor-vol-text tabular-nums">{micMonitorVolume}%</span>
          </div>
        {/if}
      </div>

      <button class="browse-btn" aria-label={$t('mic.browse_presets_aria')} on:click={onOpenBrowser}>
        <Search size={14} />
        <span>{$t('mic.browse_presets')}</span>
      </button>
    </div>
  </div>

  <!-- Favorites Strip (1 to 7 slots) -->
  <div class="favorites-strip">
    <span class="fav-label">{$t('mic.favorites')}</span>
    <div class="fav-slots">
      {#each micPresets.slice(0, 7) as favPreset, idx}
        <button
          class="fav-pill"
          class:active={favPreset.id === currentPreset.id}
          aria-label={$t('mic.select_favorite', { idx: idx + 1, name: favPreset.name })}
          on:click={() => onChangePreset(favPreset)}
        >
          <span class="fav-idx tabular-nums">{idx + 1}</span>
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
        <AlertTriangle size={15} color="var(--color-accent-warning)" />
        <span>{$t('mic.caps_warning')}</span>
      </div>
    {/if}

    <!-- Equalizer Card -->
    <div class="card eq-card">
      <div class="card-header">
        <div class="card-title">
          <Sliders size={15} color="var(--color-channel-mic)" />
          <span>{$t('mic.eq_title')}</span>
        </div>
        <div class="card-actions">
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('mic.toggle_eq')}
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
          <span class="tone-name">{$t('mic.bass_depth')}</span>
          <input
            type="range"
            aria-label={$t('mic.bass_depth_aria')}
            aria-valuenow={currentPreset.eq.bass_db ?? 0}
            aria-valuemin="-12"
            aria-valuemax="12"
            min="-12"
            max="12"
            step="0.5"
            value={currentPreset.eq.bass_db ?? 0}
            on:input={(e) => handleToneChange('bass', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val tabular-nums">{(currentPreset.eq.bass_db ?? 0) > 0 ? '+' : ''}{currentPreset.eq.bass_db ?? 0} dB</span>
        </div>
        <div class="tone-slider-group">
          <span class="tone-name">{$t('mic.vocal_clarity')}</span>
          <input
            type="range"
            aria-label={$t('mic.vocal_clarity_aria')}
            aria-valuenow={currentPreset.eq.voice_db ?? 0}
            aria-valuemin="-12"
            aria-valuemax="12"
            min="-12"
            max="12"
            step="0.5"
            value={currentPreset.eq.voice_db ?? 0}
            on:input={(e) => handleToneChange('voice', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val tabular-nums">{(currentPreset.eq.voice_db ?? 0) > 0 ? '+' : ''}{currentPreset.eq.voice_db ?? 0} dB</span>
        </div>
        <div class="tone-slider-group">
          <span class="tone-name">{$t('mic.crisp_air')}</span>
          <input
            type="range"
            aria-label={$t('mic.crisp_air_aria')}
            aria-valuenow={currentPreset.eq.treble_db ?? 0}
            aria-valuemin="-12"
            aria-valuemax="12"
            min="-12"
            max="12"
            step="0.5"
            value={currentPreset.eq.treble_db ?? 0}
            on:input={(e) => handleToneChange('treble', Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val tabular-nums">{(currentPreset.eq.treble_db ?? 0) > 0 ? '+' : ''}{currentPreset.eq.treble_db ?? 0} dB</span>
        </div>
        <div class="tone-slider-group">
          <span class="tone-name">{$t('mic.input_gain')}</span>
          <input
            type="range"
            aria-label={$t('mic.input_gain')}
            aria-valuenow={micInputGain}
            aria-valuemin="-12"
            aria-valuemax="12"
            min="-12"
            max="12"
            step="0.5"
            value={micInputGain}
            on:input={(e) => handleInputGainChange(Number(e.currentTarget.value))}
            class="tone-slider"
          />
          <span class="tone-val tabular-nums">{micInputGain > 0 ? '+' : ''}{micInputGain} dB</span>
        </div>
      </div>
    </div>

    <!-- Bottom DSP Cards Grid -->
    <div class="dsp-grid">
      <!-- Noise Remover (RNNoise) -->
      <div class="card dsp-card" class:is-disabled={!isRnnoiseAvailable}>
        <div class="card-header">
          <div class="card-title">
            <Waves size={15} color="var(--color-channel-mic)" />
            <span>{$t('mic.noise_remover')}</span>
          </div>
          <span class="dsp-badge" class:active={isRnnoiseAvailable && noiseAmount > 0}>
            {#if !isRnnoiseAvailable}
              {$t('mic.unavailable')}
            {:else if noiseAmount > 0}
              {$t('mic.wet', { amount: noiseAmount })}
            {:else}
              {$t('mic.bypass')}
            {/if}
          </span>
        </div>
        <p class="dsp-desc">{$t('mic.noise_remover_desc')}</p>
        <div class="dsp-presets-row">
          <button class="dsp-pill" class:active={noiseAmount === 0} disabled={!isRnnoiseAvailable} on:click={() => handleNoiseChange(0)}>{$t('mic.off')}</button>
          <button class="dsp-pill" class:active={noiseAmount === 40} disabled={!isRnnoiseAvailable} on:click={() => handleNoiseChange(40)}>{$t('mic.low')}</button>
          <button class="dsp-pill" class:active={noiseAmount === 65} disabled={!isRnnoiseAvailable} on:click={() => handleNoiseChange(65)}>{$t('mic.balanced')}</button>
          <button class="dsp-pill" class:active={noiseAmount === 90} disabled={!isRnnoiseAvailable} on:click={() => handleNoiseChange(90)}>{$t('mic.max')}</button>
        </div>
        <div class="dsp-control-row">
          <span class="dsp-sublabel">{$t('mic.amount')}</span>
          <input
            type="range"
            aria-label={$t('mic.amount_aria')}
            aria-valuenow={noiseAmount}
            aria-valuemin="0"
            aria-valuemax="100"
            min="0"
            max="100"
            value={noiseAmount}
            disabled={!isRnnoiseAvailable}
            on:input={(e) => handleNoiseChange(Number(e.currentTarget.value))}
            class="dsp-slider"
          />
          <span class="dsp-val tabular-nums">{noiseAmount}%</span>
        </div>
      </div>

      <!-- Noise Gate -->
      <div class="card dsp-card">
        <div class="card-header">
          <div class="card-title">
            <Shield size={15} color="var(--color-channel-mic)" />
            <span>{$t('mic.noise_gate')}</span>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('mic.toggle_gate')}
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
        <p class="dsp-desc">{$t('mic.noise_gate_desc')}</p>
        <div class="dsp-meter-row">
          {#if meter && meter.peak_dbfs > gateThreshold && (currentPreset.noise_gate?.enabled ?? true)}
            <span class="gate-status-pill open">{$t('mic.gate_open')}</span>
          {:else}
            <span class="gate-status-pill closed">{$t('mic.gate_closed')}</span>
          {/if}
        </div>
        <div class="dsp-controls-list">
          <div class="dsp-control-row">
            <span class="dsp-sublabel">{$t('mic.threshold')}</span>
            <input
              type="range"
              aria-label={$t('mic.gate_threshold_aria')}
              aria-valuenow={gateThreshold}
              aria-valuemin="-60"
              aria-valuemax="-10"
              min="-60"
              max="-10"
              value={gateThreshold}
              on:input={(e) => handleGateThresholdChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val tabular-nums">{gateThreshold} dBFS</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">{$t('mic.attack')}</span>
            <input
              type="range"
              aria-label={$t('mic.gate_attack_aria')}
              aria-valuenow={gateAttack}
              aria-valuemin="1"
              aria-valuemax="100"
              min="1"
              max="100"
              value={gateAttack}
              on:input={(e) => handleGateAttackChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val tabular-nums">{gateAttack} ms</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">{$t('mic.release')}</span>
            <input
              type="range"
              aria-label={$t('mic.gate_release_aria')}
              aria-valuenow={gateRelease}
              aria-valuemin="10"
              aria-valuemax="1000"
              min="10"
              max="1000"
              value={gateRelease}
              on:input={(e) => handleGateReleaseChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val tabular-nums">{gateRelease} ms</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">{$t('mic.hysteresis')}</span>
            <input
              type="range"
              aria-label={$t('mic.gate_hysteresis_aria')}
              aria-valuenow={gateHysteresis}
              aria-valuemin="0"
              aria-valuemax="12"
              min="0"
              max="12"
              step="0.5"
              value={gateHysteresis}
              on:input={(e) => handleGateHysteresisChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val tabular-nums">{gateHysteresis} dB</span>
          </div>
        </div>
      </div>

      <!-- Compressor -->
      <div class="card dsp-card">
        <div class="card-header">
          <div class="card-title">
            <Volume2 size={15} color="var(--color-channel-mic)" />
            <span>{$t('mic.compressor')}</span>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('mic.toggle_compressor')}
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
        <p class="dsp-desc">{$t('mic.compressor_desc')}</p>
        <div class="dsp-meter-row">
          <span class="compressor-gr-pill" class:active={parseFloat(compReduction) > 0}>
            {parseFloat(compReduction) > 0 ? $t('mic.gain_reduction_val', { reduction: compReduction }) : $t('mic.gain_reduction_zero')}
          </span>
        </div>
        <div class="dsp-controls-list">
          <div class="dsp-control-row">
            <span class="dsp-sublabel">{$t('mic.threshold')}</span>
            <input
              type="range"
              aria-label={$t('mic.comp_threshold_aria')}
              aria-valuenow={compThreshold}
              aria-valuemin="-40"
              aria-valuemax="0"
              min="-40"
              max="0"
              value={compThreshold}
              on:input={(e) => handleCompThresholdChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val tabular-nums">{compThreshold} dBFS</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">{$t('mic.ratio')}</span>
            <input
              type="range"
              aria-label={$t('mic.comp_ratio_aria')}
              aria-valuenow={compRatio}
              aria-valuemin="1"
              aria-valuemax="20"
              min="1"
              max="20"
              step="0.5"
              value={compRatio}
              on:input={(e) => handleCompRatioChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val tabular-nums">{compRatio.toFixed(1)}:1</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">{$t('mic.makeup_gain')}</span>
            <input
              type="range"
              aria-label={$t('mic.comp_makeup_aria')}
              aria-valuenow={compMakeup}
              aria-valuemin="0"
              aria-valuemax="24"
              min="0"
              max="24"
              step="0.5"
              value={compMakeup}
              on:input={(e) => handleCompMakeupChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val tabular-nums">{compMakeup > 0 ? '+' : ''}{compMakeup.toFixed(1)} dB</span>
          </div>
          <div class="dsp-control-row">
            <span class="dsp-sublabel">{$t('mic.soft_knee')}</span>
            <input
              type="range"
              aria-label={$t('mic.comp_knee_aria')}
              aria-valuenow={compKnee}
              aria-valuemin="0"
              aria-valuemax="12"
              min="0"
              max="12"
              step="0.5"
              value={compKnee}
              on:input={(e) => handleCompKneeChange(Number(e.currentTarget.value))}
              class="dsp-slider"
            />
            <span class="dsp-val tabular-nums">{compKnee.toFixed(1)} dB</span>
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
    background: var(--color-bg-app);
    display: flex;
    flex-direction: column;
    gap: 16px;
    font-family: var(--font-sans);
  }

  .preset-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: var(--color-surface-1);
    padding: 14px 20px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--color-border-default);
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

  .header-controls {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .device-select {
    background: var(--color-surface-3);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border-default);
    padding: 8px 12px;
    font-size: 11px;
    font-weight: 500;
    font-family: var(--font-sans);
    border-radius: var(--radius-sm);
    outline: none;
    cursor: pointer;
    max-width: 200px;
    text-overflow: ellipsis;
    transition: border-color var(--transition-fast);
  }
  .device-select:hover {
    border-color: var(--color-border-bright);
  }

  .monitor-wrapper {
    display: flex;
    align-items: center;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    transition: border-color var(--transition-fast), background-color var(--transition-fast);
  }
  .monitor-wrapper:hover {
    border-color: var(--color-border-bright);
  }
  .monitor-wrapper.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand);
  }

  .monitor-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    color: var(--color-text-secondary);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: color var(--transition-fast);
  }
  .monitor-btn:hover {
    color: var(--color-text-primary);
  }
  .monitor-btn.active {
    color: var(--color-channel-mic);
  }

  .monitor-vol-box {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-inline-end: 10px;
    padding-inline-start: 2px;
    border-inline-start: 1px solid var(--color-border-bright);
  }

  .monitor-slider {
    appearance: none;
    -webkit-appearance: none;
    width: 60px;
    height: 4px;
    background: var(--color-surface-1);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }
  .monitor-slider::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 12px;
    height: 12px;
    border-radius: var(--radius-sm);
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    cursor: pointer;
    transition: border-color var(--transition-fast);
  }
  .monitor-slider:hover::-webkit-slider-thumb {
    border-color: var(--color-accent-brand);
  }

  .monitor-vol-text {
    font-family: var(--font-mono);
    font-variant-numeric: var(--tabular-nums);
    font-size: 10px;
    font-weight: 700;
    color: var(--color-channel-mic);
    width: 28px;
    text-align: end;
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
  .fav-pill.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-channel-mic);
    color: var(--color-channel-mic);
    font-weight: 700;
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
    border-color: var(--color-channel-mic);
  }
  input:checked + .toggle-slider:before {
    transform: translateX(16px);
    background-color: var(--color-channel-mic);
    border-color: var(--color-channel-mic);
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
    color: var(--color-text-secondary);
    line-height: 1.4;
    margin-bottom: 14px;
    min-height: 32px;
  }

  .dsp-badge {
    font-size: 9px;
    font-weight: 800;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    background: var(--color-surface-3);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border-default);
  }
  .dsp-badge.active {
    background: var(--color-accent-brand-subtle);
    color: var(--color-channel-mic);
    border-color: var(--color-channel-mic);
  }

  .dsp-control-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .dsp-sublabel {
    font-size: 11px;
    font-weight: 600;
    color: var(--color-text-secondary);
    width: 65px;
  }

  .dsp-slider {
    flex: 1;
    appearance: none;
    -webkit-appearance: none;
    height: 5px;
    background: var(--color-surface-3);
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }
  .dsp-slider::-webkit-slider-thumb {
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
  .dsp-slider:hover::-webkit-slider-thumb {
    border-color: var(--color-accent-brand);
  }
  .dsp-slider::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: var(--radius-sm);
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    cursor: pointer;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .dsp-slider:hover::-moz-range-thumb {
    border-color: var(--color-accent-brand);
  }

  .dsp-val {
    font-family: var(--font-mono);
    font-variant-numeric: var(--tabular-nums);
    font-size: 11px;
    font-weight: 700;
    color: var(--color-text-primary);
    width: 65px;
    text-align: end;
  }

  .dsp-presets-row {
    display: flex;
    gap: 6px;
    margin-bottom: 12px;
  }
  .dsp-pill {
    flex: 1;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-sm);
    color: var(--color-text-secondary);
    font-size: 9px;
    font-weight: 700;
    font-family: var(--font-sans);
    padding: 6px 4px;
    cursor: pointer;
    transition: color var(--transition-fast), background-color var(--transition-fast), border-color var(--transition-fast);
    text-align: center;
  }
  .dsp-pill:hover {
    color: var(--color-text-primary);
    background: var(--color-surface-hover);
    border-color: var(--color-border-bright);
  }
  .dsp-pill.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-channel-mic);
    color: var(--color-channel-mic);
  }

  .caps-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    background: var(--color-accent-warning-subtle);
    border: 1px solid rgba(245, 158, 11, 0.35);
    border-radius: var(--radius-md);
    color: #FDE68A;
    font-size: 11px;
    font-weight: 500;
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
    border-radius: var(--radius-sm);
    letter-spacing: 0.5px;
    transition: all var(--transition-fast);
  }
  .gate-status-pill.open {
    background: var(--color-accent-success-subtle);
    color: var(--color-accent-success);
    border: 1px solid var(--color-accent-success);
  }
  .gate-status-pill.closed {
    background: rgba(100, 116, 139, 0.15);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border-default);
  }

  .compressor-gr-pill {
    font-size: 10px;
    font-weight: 800;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    background: rgba(100, 116, 139, 0.12);
    color: var(--color-text-muted);
    border: 1px solid var(--color-border-default);
    font-family: var(--font-mono);
    font-variant-numeric: var(--tabular-nums);
    transition: all var(--transition-fast);
  }
  .compressor-gr-pill.active {
    background: var(--color-accent-info-subtle);
    color: var(--color-accent-info);
    border: 1px solid var(--color-accent-info);
  }
</style>
