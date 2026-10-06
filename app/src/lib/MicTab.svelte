<script lang="ts">
  import type { Preset, AudioDevice } from '../types';
  import EqGraph from './EqGraph.svelte';
  import { Waves, Shield, Sliders, Volume2, Mic } from '@lucide/svelte';

  export let currentPreset: Preset;
  export let devices: AudioDevice[] = [];
  export let preferredInputDevice: string | null = null;
  export let onChangePreset: (p: Preset) => void;
  export let onSetDevice: (type: 'sink' | 'source', deviceName: string) => void;

  let isTesting = false;
  let micGain = 100;
  
  let noiseAmount = currentPreset.noise_canceller?.amount ?? 0;
  let gateThreshold = currentPreset.noise_gate?.threshold_db ?? -40;
  let compThreshold = currentPreset.compressor?.threshold_db ?? -15;

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
    if (currentPreset.noise_gate) {
      currentPreset.noise_gate.threshold_db = val;
      onChangePreset(currentPreset);
    }
  }

  function handleCompChange(val: number) {
    compThreshold = val;
    if (currentPreset.compressor) {
      currentPreset.compressor.threshold_db = val;
      onChangePreset(currentPreset);
    }
  }
</script>

<div class="mic-view">
  <div class="header-banner">
    <div class="title-group">
      <Mic size={24} color="#f97316" />
      <h2>MICROPHONE DSP</h2>
    </div>
    <div class="header-actions">
      <select
        class="mic-device-select"
        value={preferredInputDevice || 'default'}
        on:change={(e) => onSetDevice('source', e.currentTarget.value)}
      >
        <option value="default">Default Input Device</option>
        {#each (devices || []).filter(d => d.device_type.toLowerCase() === 'source') as dev}
          <option value={dev.name}>{dev.description || dev.name}</option>
        {/each}
      </select>
      <button class="test-btn" class:active={isTesting} on:click={() => (isTesting = !isTesting)}>
        {isTesting ? 'STOP TEST' : 'TEST MIC'}
      </button>
    </div>
  </div>

  <div class="card">
    <div class="card-header">
      <div class="card-title"><Waves size={16} color="#f97316" /><span>CLEARCAST RNNOISE</span></div>
      <span class="pill">{noiseAmount > 0 ? `${noiseAmount}% WET` : 'OFF'}</span>
    </div>
    <p class="desc">Neural network background noise removal.</p>
    <input type="range" min="0" max="100" value={noiseAmount} on:input={(e) => handleNoiseChange(Number(e.currentTarget.value))} class="slider" />
  </div>

  <div class="card">
    <div class="card-header">
      <div class="card-title"><Shield size={16} color="#f97316" /><span>NOISE GATE</span></div>
      <label class="switch"><input type="checkbox" checked={currentPreset.noise_gate?.enabled ?? true} /><span class="slider-round"></span></label>
    </div>
    <p class="desc">Mutes mic completely below threshold.</p>
    <div class="row">
      <span class="lbl">Threshold</span>
      <input type="range" min="-60" max="-10" value={gateThreshold} on:input={(e) => handleGateChange(Number(e.currentTarget.value))} class="slider" />
      <span class="val">{gateThreshold} dBFS</span>
    </div>
  </div>

  <div class="card">
    <div class="card-header">
      <div class="card-title"><Sliders size={16} color="#f97316" /><span>10-BAND EQUALIZER</span></div>
    </div>
    <EqGraph eqConfig={currentPreset.eq} accentColor="#f97316" onChange={(eq) => { currentPreset.eq = eq; onChangePreset(currentPreset); }} />
  </div>

  <div class="card">
    <div class="card-header">
      <div class="card-title"><Volume2 size={16} color="#f97316" /><span>DYNAMIC COMPRESSOR</span></div>
    </div>
    <p class="desc">Evens out loud yells and quiet whispers.</p>
    <div class="row">
      <span class="lbl">Threshold</span>
      <input type="range" min="-40" max="0" value={compThreshold} on:input={(e) => handleCompChange(Number(e.currentTarget.value))} class="slider" />
      <span class="val">{compThreshold} dBFS</span>
    </div>
  </div>
</div>

<style>
  .mic-view { height: 100%; overflow-y: auto; padding: 20px; background: #0d1117; display: flex; flex-direction: column; gap: 16px; font-family: 'Inter', sans-serif; }
  .header-banner { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .header-actions { display: flex; align-items: center; gap: 10px; }
  .mic-device-select { background: #161b22; color: #c9d1d9; border: 1px solid #30363d; padding: 6px 10px; font-size: 11px; border-radius: 2px; outline: none; max-width: 260px; text-overflow: ellipsis; }
  .title-group { display: flex; align-items: center; gap: 12px; color: #f8fafc; }
  .title-group h2 { font-size: 16px; font-weight: 800; letter-spacing: 0.5px; margin: 0; }
  .test-btn { background: #0d1117; border: 1px solid #30363d; color: #c9d1d9; padding: 6px 12px; font-size: 11px; font-weight: 700; border-radius: 2px; cursor: pointer; }
  .test-btn:hover { background: #1f242c; }
  .test-btn.active { background: #3b1818; border-color: #f85149; color: #f85149; }
  
  .card { background: #161b22; border: 1px solid #30363d; border-radius: 4px; padding: 16px; }
  .card-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .card-title { display: flex; align-items: center; gap: 8px; font-size: 12px; font-weight: 700; color: #c9d1d9; }
  .desc { font-size: 11px; color: #8b949e; margin-bottom: 12px; }
  .pill { background: #0d1117; border: 1px solid #f9731644; color: #f97316; font-size: 10px; padding: 2px 6px; border-radius: 2px; font-weight: 700; }
  
  .row { display: flex; align-items: center; gap: 12px; }
  .lbl { font-size: 11px; color: #8b949e; width: 80px; }
  .val { font-size: 11px; color: #c9d1d9; font-family: monospace; font-weight: 700; width: 60px; text-align: right; }
  
  .slider { flex: 1; height: 4px; background: #0d1117; border: 1px solid #30363d; outline: none; appearance: none; border-radius: 2px; }
  .slider::-webkit-slider-thumb { appearance: none; width: 14px; height: 14px; background: #f97316; border-radius: 2px; cursor: pointer; }
  
  .switch { position: relative; display: inline-block; width: 32px; height: 16px; }
  .switch input { opacity: 0; width: 0; height: 0; }
  .slider-round { position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: #0d1117; border: 1px solid #30363d; transition: .2s; border-radius: 2px; }
  .slider-round:before { position: absolute; content: ""; height: 10px; width: 10px; left: 2px; bottom: 2px; background-color: #8b949e; transition: .2s; border-radius: 1px; }
  input:checked + .slider-round { background-color: #f9731622; border-color: #f97316; }
  input:checked + .slider-round:before { transform: translateX(16px); background-color: #f97316; }
</style>
