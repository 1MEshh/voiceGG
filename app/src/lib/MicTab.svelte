<script lang="ts">
  import type { Preset, AudioDevice } from '../types';
  import EqGraph from './EqGraph.svelte';
  import { Mic, Waves, Shield, Disc, Sliders, Volume2, Sparkles } from '@lucide/svelte';

  export let currentPreset: Preset;
  export let devices: AudioDevice[];
  export let preferredInputDevice: string | null = null;
  export let onChangePreset: (preset: Preset) => void;
  export let onSetDevice: (type: 'sink' | 'source', deviceName: string) => void;

  let micGain = 100;
  let noiseAmount = currentPreset.noise_canceller?.amount ?? 75;
  let gateThreshold = currentPreset.noise_gate?.threshold_db ?? -42;
  let compThreshold = currentPreset.compressor?.threshold_db ?? -18;
  let compRatio = currentPreset.compressor?.ratio ?? 3;
  let isTestingMic = false;

  function handleNoiseChange(val: number) {
    noiseAmount = val;
    if (!currentPreset.noise_canceller) {
      currentPreset.noise_canceller = { enabled: val > 0, amount: val, vad_threshold: 0.5 };
    } else {
      currentPreset.noise_canceller.amount = val;
      currentPreset.noise_canceller.enabled = val > 0;
    }
    onChangePreset(currentPreset);
  }

  function handleGateChange(val: number) {
    gateThreshold = val;
    if (!currentPreset.noise_gate) {
      currentPreset.noise_gate = { enabled: true, threshold_db: val, attack_ms: 2, hold_ms: 50, release_ms: 80 };
    } else {
      currentPreset.noise_gate.threshold_db = val;
    }
    onChangePreset(currentPreset);
  }
</script>

<div class="mic-view">
  <!-- Input Device & Gain Header Card -->
  <div class="card header-card">
    <div class="card-header">
      <div class="card-title">
        <Mic size={16} color="#f97316" />
        <span>MICROPHONE INPUT & GAIN</span>
      </div>
      <button
        class="test-btn"
        class:testing={isTestingMic}
        on:click={() => isTestingMic = !isTestingMic}
      >
        <Disc size={14} />
        <span>{isTestingMic ? 'RECORDING (3s)...' : 'MIC TEST'}</span>
      </button>
    </div>

    <div class="mic-controls-row">
      <div class="device-col">
        <span class="label">INPUT DEVICE</span>
        <select
          class="device-select"
          value={preferredInputDevice || 'default'}
          on:change={(e) => onSetDevice('source', e.currentTarget.value)}
        >
          <option value="default">Default Hardware Source</option>
          {#each (devices || []).filter(d => d.device_type.toLowerCase() === 'source' && !d.name.endsWith('.monitor')) as dev}
            <option value={dev.name}>{dev.description || dev.name}</option>
          {/each}
        </select>
      </div>

      <div class="gain-col">
        <div class="label-row">
          <span class="label">INPUT GAIN</span>
          <span class="val-mono">{micGain}%</span>
        </div>
        <input type="range" min="0" max="150" bind:value={micGain} class="slider" />
      </div>

      <div class="meter-col">
        <span class="label">INPUT LEVEL</span>
        <div class="mic-vu-bar">
          <div class="mic-vu-fill" style="width: 58%;"></div>
        </div>
      </div>
    </div>
  </div>

  <!-- AI Noise Cancellation Card (ClearCast RNNoise) -->
  <div class="card noise-card">
    <div class="card-header">
      <div class="card-title">
        <Waves size={16} color="#f97316" />
        <span>CLEARCAST AI NOISE REMOVAL (RNNOISE)</span>
      </div>
      <span class="pill-badge">{noiseAmount > 0 ? `${noiseAmount}% WET` : 'OFF'}</span>
    </div>
    <p class="card-desc">
      Neural network speech enhancement trained on mechanical keyboards, fan noise, and background room acoustics.
    </p>

    <div class="slider-control-block">
      <input
        type="range"
        min="0"
        max="100"
        value={noiseAmount}
        on:input={(e) => handleNoiseChange(Number(e.currentTarget.value))}
        class="big-slider"
      />
      <div class="slider-ticks">
        <span class:active-tick={noiseAmount === 0}>OFF</span>
        <span class:active-tick={noiseAmount > 0 && noiseAmount <= 50}>LIGHT (25%)</span>
        <span class:active-tick={noiseAmount > 50 && noiseAmount <= 80}>STUDIO (75%)</span>
        <span class:active-tick={noiseAmount > 80}>MAX (100%)</span>
      </div>
    </div>
  </div>

  <!-- Noise Gate Card -->
  <div class="card">
    <div class="card-header">
      <div class="card-title">
        <Shield size={16} color="#f97316" />
        <span>NOISE GATE</span>
      </div>
      <label class="toggle-switch">
        <input type="checkbox" checked={currentPreset.noise_gate?.enabled ?? true} />
        <span class="toggle-slider"></span>
      </label>
    </div>
    <p class="card-desc">
      Mutes your microphone completely when you are not speaking to eliminate room breathing and PC fans.
    </p>
    <div class="control-row">
      <span class="control-label">Gate Threshold</span>
      <input
        type="range"
        min="-60"
        max="-10"
        value={gateThreshold}
        on:input={(e) => handleGateChange(Number(e.currentTarget.value))}
        class="slider"
      />
      <span class="val-mono">{gateThreshold} dBFS</span>
    </div>
  </div>

  <!-- Mic Equalizer Card -->
  <div class="card">
    <div class="card-header">
      <div class="card-title">
        <Sliders size={16} color="#f97316" />
        <span>MICROPHONE 10-BAND EQUALIZER</span>
      </div>
    </div>
    <EqGraph
      eqConfig={currentPreset.eq}
      accentColor="#f97316"
      onChange={(eq) => {
        currentPreset.eq = eq;
        onChangePreset(currentPreset);
      }}
    />
  </div>

  <!-- Compressor Card -->
  <div class="card">
    <div class="card-header">
      <div class="card-title">
        <Volume2 size={16} color="#f97316" />
        <span>DYNAMIC COMPRESSOR & CLIP GUARD</span>
      </div>
    </div>
    <p class="card-desc">
      Evens out loud yells and quiet whispers. Built-in hard safety peak limiter prevents distortion clipping.
    </p>
    <div class="control-row">
      <span class="control-label">Threshold</span>
      <input type="range" min="-40" max="0" bind:value={compThreshold} class="slider" />
      <span class="val-mono">{compThreshold} dBFS</span>
    </div>
  </div>
</div>

<style>
  .mic-view {
    height: calc(100vh - 52px);
    overflow-y: auto;
    padding: 16px 24px;
    background: #0d1117;
    display: flex;
    flex-direction: column;
    gap: 16px;
    user-select: none;
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

  .card-desc {
    font-size: 12px;
    color: #94a3b8;
    margin-bottom: 12px;
    line-height: 1.4;
  }

  .test-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    background: #202733;
    border: 1px solid #2d3846;
    border-radius: 5px;
    color: #cbd5e1;
    font-size: 11px;
    font-weight: 700;
    cursor: pointer;
  }

  .test-btn.testing {
    background: #3b1818;
    border-color: #ef4444;
    color: #ef4444;
  }

  .mic-controls-row {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 16px;
    align-items: center;
  }

  .device-col, .gain-col, .meter-col {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .label-row {
    display: flex;
    justify-content: space-between;
  }

  .label {
    font-size: 10px;
    font-weight: 700;
    color: #64748b;
    letter-spacing: 0.5px;
  }

  .val-mono {
    font-family: monospace;
    font-size: 12px;
    font-weight: 700;
    color: #f1f5f9;
  }

  .device-select {
    background: #11151c;
    border: 1px solid #232b36;
    border-radius: 5px;
    color: #cbd5e1;
    font-size: 11px;
    padding: 6px 8px;
    outline: none;
  }

  .mic-vu-bar {
    height: 10px;
    background: #11151c;
    border-radius: 4px;
    overflow: hidden;
    border: 1px solid #232b36;
  }

  .mic-vu-fill {
    height: 100%;
    background: linear-gradient(to right, #22c55e 70%, #eab308 90%, #ef4444 100%);
    border-radius: 3px;
  }

  .pill-badge {
    padding: 3px 8px;
    background: #f9731622;
    border: 1px solid #f9731666;
    border-radius: 12px;
    color: #fb923c;
    font-size: 11px;
    font-weight: 700;
    font-family: monospace;
  }

  .slider-control-block {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .big-slider {
    width: 100%;
    appearance: none;
    height: 8px;
    background: #232b36;
    border-radius: 4px;
    outline: none;
    cursor: pointer;
  }

  .big-slider::-webkit-slider-thumb {
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #f97316;
    box-shadow: 0 0 8px rgba(249, 115, 22, 0.6);
    cursor: pointer;
  }

  .slider-ticks {
    display: flex;
    justify-content: space-between;
    font-size: 10px;
    font-weight: 700;
    color: #64748b;
  }

  .active-tick {
    color: #fb923c;
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

  .slider {
    flex: 1;
    appearance: none;
    height: 6px;
    background: #232b36;
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }

  .slider::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #f1f5f9;
    cursor: pointer;
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
    background-color: #f97316;
  }

  input:checked + .toggle-slider:before {
    transform: translateX(16px);
  }
</style>
