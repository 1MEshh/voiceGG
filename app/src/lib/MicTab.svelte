<script lang="ts">
  import { onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { Preset, AudioDevice, PluginCapabilities, ChannelMeter, ChannelId } from '../types';
  import EqGraph from './EqGraph.svelte';
  import SonarSlider from './SonarSlider.svelte';
  import CustomSelect, { type SelectOption } from './CustomSelect.svelte';
  import {
    Waves,
    Shield,
    Sliders,
    Volume2,
    Mic,
    Search,
    Headphones,
    AlertTriangle,
    BookmarkPlus,
    RotateCcw,
  } from '@lucide/svelte';
  import { t } from './i18n';

  export let currentPreset: Preset;
  export let allPresets: Preset[] = [];
  export let builtinPresets: Preset[] = [];
  export let devices: AudioDevice[] = [];
  export let preferredInputDevice: string | null = null;
  export let onChangePreset: (p: Preset) => void;
  export let onSelectPreset: ((p: Preset) => void) | undefined = undefined;
  export let onResetPreset: ((channel: ChannelId, presetId: string) => void) | undefined = undefined;
  export let onOpenBrowser: () => void = () => {};
  export let onSetDevice: (type: 'sink' | 'source', deviceName: string) => void;
  export let micMonitorEnabled: boolean = false;
  export let micMonitorVolume: number = 50;
  export let capabilities: PluginCapabilities | undefined = undefined;
  export let meter: ChannelMeter | undefined = undefined;
  export let onSetMicMonitor: (enabled: boolean, volume: number) => void;
  export let onSaveCustomPreset: ((channel: ChannelId, preset: Preset) => void) | undefined = undefined;

  const MIC_ACCENT = '#FF5200';

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
    if (!copy.eq) {
      copy.eq = {
        enabled: true,
        bands: [],
        bass_db: 0,
        voice_db: 0,
        treble_db: 0,
        input_gain_db: 0,
      };
    }
    if (copy.eq.input_gain_db === undefined) {
      copy.eq.input_gain_db = 0;
    }
    return copy;
  }

  $: if (
    currentPreset &&
    (currentPreset.id !== lastIncomingPresetId || Date.now() - lastLocalEditAt > 600)
  ) {
    lastIncomingPresetId = currentPreset.id;
    localPreset = clonePreset(currentPreset);
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
      onChangePreset(clonePreset(localPreset));
    } else if (!saveThrottleTimer) {
      const remaining = 40 - (now - lastSaveAt);
      saveThrottleTimer = setTimeout(() => {
        saveThrottleTimer = null;
        lastSaveAt = Date.now();
        onChangePreset(clonePreset(localPreset));
      }, remaining);
    }
  }

  onDestroy(() => {
    if (saveThrottleTimer) clearTimeout(saveThrottleTimer);
  });

  // Built-in Rust nnnoiseless crate is always available, honoring capability flags if provided
  $: isRnnoiseAvailable = capabilities?.rnnoise ?? true;

  $: factoryPreset = builtinPresets.find((bp) => bp.id === localPreset.id);
  $: isModified = Boolean(
    factoryPreset &&
      (JSON.stringify(localPreset.eq) !== JSON.stringify(factoryPreset.eq) ||
        JSON.stringify(localPreset.noise_canceller) !== JSON.stringify(factoryPreset.noise_canceller) ||
        JSON.stringify(localPreset.compressor) !== JSON.stringify(factoryPreset.compressor) ||
        JSON.stringify(localPreset.noise_gate) !== JSON.stringify(factoryPreset.noise_gate))
  );

  async function handleResetToFactory() {
    if (factoryPreset) {
      localPreset = clonePreset(factoryPreset);
      lastIncomingPresetId = factoryPreset.id;
    }
    if (onResetPreset) {
      onResetPreset('mic', localPreset.id);
    } else {
      try {
        await invoke('reset_preset', { channel: 'mic', presetId: localPreset.id });
      } catch {
        if (factoryPreset) onChangePreset(clonePreset(factoryPreset));
      }
    }
  }

  $: noiseAmount = localPreset.noise_canceller?.amount ?? 0;
  $: gateThreshold = localPreset.noise_gate?.threshold_db ?? -40;
  $: gateAttack = localPreset.noise_gate?.attack_ms ?? 10;
  $: gateRelease = localPreset.noise_gate?.release_ms ?? 100;
  $: gateHysteresis = localPreset.noise_gate?.hysteresis_db ?? 6;

  $: compThreshold = localPreset.compressor?.threshold_db ?? -18;
  $: compRatio = localPreset.compressor?.ratio ?? 3;
  $: compMakeup = localPreset.compressor?.makeup_gain_db ?? 2;
  $: compKnee = localPreset.compressor?.knee_db ?? 3;

  // Real Gain Reduction calculation
  $: compReduction = (() => {
    if (!meter || !(localPreset.compressor?.enabled ?? true)) return '0.0';
    if (meter.peak_dbfs > compThreshold && compRatio > 1) {
      const red = (meter.peak_dbfs - compThreshold) * (1 - 1 / compRatio);
      return red > 0 ? red.toFixed(1) : '0.0';
    }
    return '0.0';
  })();

  $: deviceOptions = (() => {
    const list = (devices || []).filter((d) => d.device_type.toLowerCase() === 'source');
    const options: SelectOption[] = [
      { value: 'default', label: $t('mic.default_input') },
      ...list.map((d) => ({
        value: d.name,
        label: d.description || d.name,
      })),
    ];
    if (
      preferredInputDevice &&
      preferredInputDevice !== 'default' &&
      !list.some((d) => d.name === preferredInputDevice)
    ) {
      options.push({
        value: preferredInputDevice,
        label: `${preferredInputDevice} (${$t('mixer.device_disconnected', { default: 'Disconnected — Using System Default' })})`,
      });
    }
    return options;
  })();

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
      description: `Custom tuned microphone preset`,
      category: 'custom',
      tags: ['mic', 'custom'],
    };
    if (onSaveCustomPreset) {
      onSaveCustomPreset('mic', newPreset);
    }
    isSavingCustom = false;
  }

  function handleNoiseChange(val: number, immediate = false) {
    if (!localPreset.noise_canceller) {
      localPreset.noise_canceller = { enabled: val > 0, amount: val, vad_threshold: 0.35 };
    } else {
      localPreset.noise_canceller.amount = val;
      localPreset.noise_canceller.enabled = val > 0;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(immediate);
  }

  function handleGateThresholdChange(val: number) {
    if (!localPreset.noise_gate) {
      localPreset.noise_gate = {
        enabled: true,
        threshold_db: val,
        attack_ms: 10,
        hold_ms: 50,
        release_ms: 100,
      };
    } else {
      localPreset.noise_gate.threshold_db = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleGateAttackChange(val: number) {
    if (!localPreset.noise_gate) {
      localPreset.noise_gate = {
        enabled: true,
        threshold_db: -40,
        attack_ms: val,
        hold_ms: 50,
        release_ms: 100,
      };
    } else {
      localPreset.noise_gate.attack_ms = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleGateReleaseChange(val: number) {
    if (!localPreset.noise_gate) {
      localPreset.noise_gate = {
        enabled: true,
        threshold_db: -40,
        attack_ms: 10,
        hold_ms: 50,
        release_ms: val,
      };
    } else {
      localPreset.noise_gate.release_ms = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleGateHysteresisChange(val: number) {
    if (!localPreset.noise_gate) {
      localPreset.noise_gate = {
        enabled: true,
        threshold_db: -40,
        attack_ms: 10,
        hold_ms: 50,
        release_ms: 100,
        hysteresis_db: val,
      };
    } else {
      localPreset.noise_gate.hysteresis_db = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleCompThresholdChange(val: number) {
    if (!localPreset.compressor) {
      localPreset.compressor = {
        enabled: true,
        threshold_db: val,
        ratio: 3,
        attack_ms: 10,
        release_ms: 100,
        makeup_gain_db: 2,
      };
    } else {
      localPreset.compressor.threshold_db = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleCompRatioChange(val: number) {
    if (!localPreset.compressor) {
      localPreset.compressor = {
        enabled: true,
        threshold_db: -18,
        ratio: val,
        attack_ms: 10,
        release_ms: 100,
        makeup_gain_db: 2,
      };
    } else {
      localPreset.compressor.ratio = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleCompMakeupChange(val: number) {
    if (!localPreset.compressor) {
      localPreset.compressor = {
        enabled: true,
        threshold_db: -18,
        ratio: 3,
        attack_ms: 10,
        release_ms: 100,
        makeup_gain_db: val,
      };
    } else {
      localPreset.compressor.makeup_gain_db = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleCompKneeChange(val: number) {
    if (!localPreset.compressor) {
      localPreset.compressor = {
        enabled: true,
        threshold_db: -18,
        ratio: 3,
        attack_ms: 10,
        release_ms: 100,
        makeup_gain_db: 2,
        knee_db: val,
      };
    } else {
      localPreset.compressor.knee_db = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleToneChange(type: 'bass' | 'voice' | 'treble', val: number) {
    if (type === 'bass') localPreset.eq.bass_db = val;
    if (type === 'voice') localPreset.eq.voice_db = val;
    if (type === 'treble') localPreset.eq.treble_db = val;
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  function handleInputGainChange(val: number) {
    if (!localPreset.eq) {
      localPreset.eq = {
        enabled: true,
        bands: [],
        bass_db: 0,
        voice_db: 0,
        treble_db: 0,
        input_gain_db: val,
      };
    } else {
      localPreset.eq.input_gain_db = val;
    }
    localPreset = { ...localPreset };
    scheduleSavePreset(false);
  }

  $: micPresets = allPresets.filter((p) => {
    if (p.category === 'mic') return true;
    if (p.category === 'custom') {
      if (p.tags?.includes('mic')) return true;
      if (p.description?.toLowerCase().includes('mic') || p.description?.toLowerCase().includes('microphone')) return true;
      if (p.noise_gate || p.compressor) return true;
      return false;
    }
    return false;
  });
</script>

<div class="channel-view mic-view">
  <!-- Preset Bar -->
  <div class="preset-bar">
    <div class="preset-info">
      <div class="preset-icon-badge">
        <Mic size={18} color={MIC_ACCENT} />
      </div>
      <div class="preset-titles">
        <div class="preset-title-row">
          <span class="preset-name">{localPreset.name}</span>
          <span class="preset-ch-tag">MICROPHONE</span>
        </div>
        <span class="preset-desc">{localPreset.description}</span>
      </div>
    </div>

    <div class="header-controls">
      <!-- Input Device Selector -->
      <div class="device-select-container">
        <CustomSelect
          value={preferredInputDevice || 'default'}
          options={deviceOptions}
          placeholder={$t('mic.default_input')}
          onChange={(val) => onSetDevice('source', val)}
        />
      </div>

      <!-- MIC SIDETONE (VOICE MONITOR) -->
      <div class="monitor-wrapper" class:active={micMonitorEnabled}>
        <button
          type="button"
          class="monitor-btn"
          class:active={micMonitorEnabled}
          aria-label={micMonitorEnabled ? $t('mic.mute_loopback') : $t('mic.listen_realtime')}
          on:click={() => onSetMicMonitor(!micMonitorEnabled, micMonitorVolume)}
          title="MIC SIDETONE (VOICE MONITOR)"
        >
          <Headphones size={13} />
          <span>MIC SIDETONE (VOICE MONITOR)</span>
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
          title="Save current settings as a new custom preset"
        >
          <BookmarkPlus size={14} />
          <span>Save Custom</span>
        </button>
        <button
          type="button"
          class="browse-btn"
          aria-label={$t('mic.browse_presets_aria')}
          on:click={onOpenBrowser}
        >
          <Search size={14} />
          <span>{$t('mic.browse_presets')}</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Presets Quick-Select Strip -->
  <div class="favorites-strip">
    <span class="fav-label">PRESETS:</span>
    <div class="fav-slots">
      {#each micPresets.slice(0, 7) as favPreset, idx}
        <button
          type="button"
          class="fav-pill"
          class:active={favPreset.id === localPreset.id}
          aria-label={$t('mic.select_favorite', { idx: idx + 1, name: favPreset.name })}
          on:click={() => {
            lastIncomingPresetId = favPreset.id;
            localPreset = clonePreset(favPreset);
            if (onSelectPreset) {
              onSelectPreset(favPreset);
            } else {
              onChangePreset(favPreset);
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

    <!-- Equalizer & VOICE TONE Card -->
    <div class="card eq-card">
      <div class="card-header">
        <div class="card-title">
          <Sliders size={15} color={MIC_ACCENT} />
          <span>{$t('mic.eq_title')}</span>
        </div>
        <div class="card-actions">
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('mic.toggle_eq')}
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
        accentColor={MIC_ACCENT}
        onChange={(eq) => {
          localPreset.eq = eq;
          localPreset = { ...localPreset };
          scheduleSavePreset(false);
        }}
      />

      <!-- VOICE TONE Section -->
      <div class="voice-tone-section">
        <div class="voice-tone-header">
          <span class="voice-tone-title">VOICE TONE</span>
        </div>
        <div class="quick-tone-grid">
          <SonarSlider
            label={$t('mic.bass_depth')}
            min={-12}
            max={12}
            step={0.5}
            unit="dB"
            centerDetent={true}
            defaultValue={0}
            accentColor={MIC_ACCENT}
            value={localPreset.eq.bass_db ?? 0}
            disabled={!localPreset.eq.enabled}
            onChange={(val) => handleToneChange('bass', val)}
          />
          <SonarSlider
            label={$t('mic.vocal_clarity')}
            min={-12}
            max={12}
            step={0.5}
            unit="dB"
            centerDetent={true}
            defaultValue={0}
            accentColor={MIC_ACCENT}
            value={localPreset.eq.voice_db ?? 0}
            disabled={!localPreset.eq.enabled}
            onChange={(val) => handleToneChange('voice', val)}
          />
          <SonarSlider
            label={$t('mic.crisp_air')}
            min={-12}
            max={12}
            step={0.5}
            unit="dB"
            centerDetent={true}
            defaultValue={0}
            accentColor={MIC_ACCENT}
            value={localPreset.eq.treble_db ?? 0}
            disabled={!localPreset.eq.enabled}
            onChange={(val) => handleToneChange('treble', val)}
          />
          <SonarSlider
            label={$t('mic.input_gain')}
            min={-12}
            max={12}
            step={0.5}
            unit="dB"
            centerDetent={true}
            defaultValue={0}
            accentColor={MIC_ACCENT}
            value={localPreset.eq?.input_gain_db ?? 0}
            disabled={false}
            onChange={(val) => handleInputGainChange(val)}
          />
        </div>
      </div>
    </div>

    <!-- Bottom DSP Cards Grid -->
    <div class="dsp-grid">
      <!-- CLEARCAST AI NOISE CANCELLATION -->
      <div class="card dsp-card" class:is-disabled={!isRnnoiseAvailable}>
        <div class="card-header">
          <div class="card-title">
            <Waves size={15} color={MIC_ACCENT} />
            <span>CLEARCAST AI NOISE CANCELLATION</span>
          </div>
          <div class="card-status-badges">
            <span class="dsp-badge" class:active={isRnnoiseAvailable && noiseAmount > 0}>
              {#if !isRnnoiseAvailable}
                {$t('mic.unavailable')}
              {:else if noiseAmount > 0}
                {noiseAmount}% ACTIVE
              {:else}
                {$t('mic.bypass')}
              {/if}
            </span>
          </div>
        </div>
        <p class="dsp-desc">{$t('mic.noise_remover_desc')}</p>
        <div class="dsp-presets-row">
          <button
            type="button"
            class="dsp-pill"
            class:active={noiseAmount === 0}
            disabled={!isRnnoiseAvailable}
            on:click={() => handleNoiseChange(0, true)}>{$t('mic.off')}</button
          >
          <button
            type="button"
            class="dsp-pill"
            class:active={noiseAmount === 40}
            disabled={!isRnnoiseAvailable}
            on:click={() => handleNoiseChange(40, true)}>{$t('mic.low')}</button
          >
          <button
            type="button"
            class="dsp-pill"
            class:active={noiseAmount === 65}
            disabled={!isRnnoiseAvailable}
            on:click={() => handleNoiseChange(65, true)}>{$t('mic.balanced')}</button
          >
          <button
            type="button"
            class="dsp-pill"
            class:active={noiseAmount === 100}
            disabled={!isRnnoiseAvailable}
            on:click={() => handleNoiseChange(100, true)}>{$t('mic.max')}</button
          >
        </div>
        <div class="dsp-slider-wrapper">
          <SonarSlider
            label={$t('mic.amount')}
            min={0}
            max={100}
            step={1}
            unit="%"
            accentColor={MIC_ACCENT}
            value={noiseAmount}
            disabled={!isRnnoiseAvailable}
            onChange={(val) => handleNoiseChange(val, false)}
          />
        </div>
      </div>

      <!-- NOISE GATE -->
      <div class="card dsp-card">
        <div class="card-header">
          <div class="card-title">
            <Shield size={15} color={MIC_ACCENT} />
            <span>NOISE GATE</span>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('mic.toggle_gate')}
              checked={localPreset.noise_gate?.enabled ?? true}
              on:change={(e) => {
                if (!localPreset.noise_gate) {
                  localPreset.noise_gate = {
                    enabled: e.currentTarget.checked,
                    threshold_db: -40,
                    attack_ms: 10,
                    hold_ms: 50,
                    release_ms: 100,
                  };
                } else {
                  localPreset.noise_gate.enabled = e.currentTarget.checked;
                }
                localPreset = { ...localPreset };
                scheduleSavePreset(true);
              }}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
        <p class="dsp-desc">{$t('mic.noise_gate_desc')}</p>
        <div class="dsp-meter-row">
          {#if meter && meter.peak_dbfs > gateThreshold && (localPreset.noise_gate?.enabled ?? true)}
            <span class="gate-status-pill open">{$t('mic.gate_open')}</span>
          {:else}
            <span class="gate-status-pill closed">{$t('mic.gate_closed')}</span>
          {/if}
        </div>
        <div class="dsp-controls-list">
          <SonarSlider
            label={$t('mic.threshold')}
            min={-60}
            max={-10}
            step={1}
            unit="dBFS"
            accentColor={MIC_ACCENT}
            value={gateThreshold}
            disabled={!(localPreset.noise_gate?.enabled ?? true)}
            onChange={(val) => handleGateThresholdChange(val)}
          />
          <SonarSlider
            label={$t('mic.attack')}
            min={1}
            max={100}
            step={1}
            unit="ms"
            accentColor={MIC_ACCENT}
            value={gateAttack}
            disabled={!(localPreset.noise_gate?.enabled ?? true)}
            onChange={(val) => handleGateAttackChange(val)}
          />
          <SonarSlider
            label={$t('mic.release')}
            min={10}
            max={1000}
            step={5}
            unit="ms"
            accentColor={MIC_ACCENT}
            value={gateRelease}
            disabled={!(localPreset.noise_gate?.enabled ?? true)}
            onChange={(val) => handleGateReleaseChange(val)}
          />
          <SonarSlider
            label={$t('mic.hysteresis')}
            min={0}
            max={12}
            step={0.5}
            unit="dB"
            accentColor={MIC_ACCENT}
            value={gateHysteresis}
            disabled={!(localPreset.noise_gate?.enabled ?? true)}
            onChange={(val) => handleGateHysteresisChange(val)}
          />
        </div>
      </div>

      <!-- COMPRESSOR (VOICE STABILIZER) -->
      <div class="card dsp-card">
        <div class="card-header">
          <div class="card-title">
            <Volume2 size={15} color={MIC_ACCENT} />
            <span>COMPRESSOR (VOICE STABILIZER)</span>
          </div>
          <label class="toggle-switch">
            <input
              type="checkbox"
              aria-label={$t('mic.toggle_compressor')}
              checked={localPreset.compressor?.enabled ?? true}
              on:change={(e) => {
                if (!localPreset.compressor) {
                  localPreset.compressor = {
                    enabled: e.currentTarget.checked,
                    threshold_db: -18,
                    ratio: 3,
                    attack_ms: 10,
                    release_ms: 100,
                    makeup_gain_db: 2,
                  };
                } else {
                  localPreset.compressor.enabled = e.currentTarget.checked;
                }
                localPreset = { ...localPreset };
                scheduleSavePreset(true);
              }}
            />
            <span class="toggle-slider"></span>
          </label>
        </div>
        <p class="dsp-desc">{$t('mic.compressor_desc')}</p>
        <div class="dsp-meter-row">
          <span class="compressor-gr-pill" class:active={parseFloat(compReduction) > 0}>
            {parseFloat(compReduction) > 0
              ? $t('mic.gain_reduction_val', { reduction: compReduction })
              : $t('mic.gain_reduction_zero')}
          </span>
        </div>
        <div class="dsp-controls-list">
          <SonarSlider
            label={$t('mic.threshold')}
            min={-40}
            max={0}
            step={1}
            unit="dBFS"
            accentColor={MIC_ACCENT}
            value={compThreshold}
            disabled={!(localPreset.compressor?.enabled ?? true)}
            onChange={(val) => handleCompThresholdChange(val)}
          />
          <SonarSlider
            label={$t('mic.ratio')}
            min={1}
            max={20}
            step={0.5}
            unit=":1"
            accentColor={MIC_ACCENT}
            value={compRatio}
            disabled={!(localPreset.compressor?.enabled ?? true)}
            onChange={(val) => handleCompRatioChange(val)}
          />
          <SonarSlider
            label={$t('mic.makeup_gain')}
            min={0}
            max={24}
            step={0.5}
            unit="dB"
            accentColor={MIC_ACCENT}
            value={compMakeup}
            disabled={!(localPreset.compressor?.enabled ?? true)}
            onChange={(val) => handleCompMakeupChange(val)}
          />
          <SonarSlider
            label={$t('mic.soft_knee')}
            min={0}
            max={12}
            step={0.5}
            unit="dB"
            accentColor={MIC_ACCENT}
            value={compKnee}
            disabled={!(localPreset.compressor?.enabled ?? true)}
            onChange={(val) => handleCompKneeChange(val)}
          />
        </div>
      </div>
    </div>
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
        <p class="modal-desc">Save current microphone EQ curve and DSP tuning into your custom preset library.</p>
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
    padding: 12px 18px;
    border-radius: var(--radius-md, 6px);
    border: 1px solid var(--color-border-default, #232834);
    flex-wrap: wrap;
    gap: 12px;
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
    border: 1px solid rgba(255, 82, 0, 0.35);
    color: var(--color-channel-mic, #FF5200);
    letter-spacing: 0.5px;
  }

  .preset-desc {
    font-size: 11px;
    color: var(--color-text-secondary, #94A3B8);
  }

  .header-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }

  .monitor-wrapper {
    display: flex;
    align-items: center;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    transition: border-color var(--transition-fast), background-color var(--transition-fast);
  }

  .monitor-wrapper:hover {
    border-color: var(--color-border-bright, #2F3646);
  }

  .monitor-wrapper.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand, #FF5200);
  }

  .monitor-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 11px;
    background: transparent;
    border: none;
    color: var(--color-text-secondary, #94A3B8);
    font-size: 10px;
    font-weight: 800;
    font-family: var(--font-sans);
    letter-spacing: 0.4px;
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  .monitor-btn:hover {
    color: var(--color-text-primary, #F8FAFC);
  }

  .monitor-btn.active {
    color: var(--color-channel-mic, #FF5200);
  }

  .monitor-vol-box {
    display: flex;
    align-items: center;
    gap: 6px;
    padding-inline-end: 10px;
    padding-inline-start: 6px;
    border-inline-start: 1px solid var(--color-border-bright, #2F3646);
  }

  .monitor-slider {
    appearance: none;
    -webkit-appearance: none;
    width: 60px;
    height: 4px;
    background: var(--color-neutral-950, #08090C);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }

  .monitor-slider::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 12px;
    height: 12px;
    border-radius: var(--radius-xs, 3px);
    background: var(--color-neutral-200, #E2E8F0);
    border: 1px solid var(--color-border-bright, #2F3646);
    cursor: pointer;
  }

  .monitor-vol-text {
    font-family: var(--font-mono);
    font-variant-numeric: var(--tabular-nums);
    font-size: 10px;
    font-weight: 700;
    color: var(--color-channel-mic, #FF5200);
    width: 28px;
    text-align: end;
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

  .fav-pill.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-channel-mic, #FF5200);
    color: var(--color-channel-mic, #FF5200);
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
    border-color: var(--color-channel-mic, #FF5200);
  }

  input:checked + .toggle-slider:before {
    transform: translateX(16px);
    background-color: var(--color-channel-mic, #FF5200);
  }

  .preset-actions-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .device-select-container {
    width: 200px;
  }

  .voice-tone-section {
    margin-top: 14px;
    padding-top: 12px;
    border-top: 1px solid var(--color-border-subtle, #1C2029);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .voice-tone-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .voice-tone-title {
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.7px;
    color: var(--color-text-secondary, #94A3B8);
    text-transform: uppercase;
  }

  .quick-tone-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 16px;
  }

  .card-status-badges {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .dsp-slider-wrapper {
    margin-top: 6px;
  }

  .dsp-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 14px;
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
    margin-bottom: 12px;
    min-height: 30px;
  }

  .dsp-badge {
    font-size: 9px;
    font-weight: 800;
    font-family: var(--font-mono);
    padding: 2px 6px;
    border-radius: var(--radius-xs, 3px);
    background: var(--color-surface-2, #171B22);
    color: var(--color-text-muted, #64748B);
    border: 1px solid var(--color-border-default, #232834);
  }

  .dsp-badge.active {
    background: var(--color-accent-brand-subtle);
    color: var(--color-channel-mic, #FF5200);
    border-color: var(--color-channel-mic, #FF5200);
  }

  .dsp-presets-row {
    display: flex;
    gap: 6px;
    margin-bottom: 12px;
  }

  .dsp-pill {
    flex: 1;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 9px;
    font-weight: 700;
    font-family: var(--font-sans);
    padding: 6px 4px;
    cursor: pointer;
    transition: all var(--transition-fast);
    text-align: center;
  }

  .dsp-pill:hover {
    color: var(--color-text-primary, #F8FAFC);
    background: var(--color-surface-hover, #262C38);
    border-color: var(--color-border-bright, #2F3646);
  }

  .dsp-pill.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-channel-mic, #FF5200);
    color: var(--color-channel-mic, #FF5200);
  }

  .dsp-card.is-disabled {
    opacity: 0.55;
  }

  .dsp-card.is-disabled .dsp-pill {
    cursor: not-allowed;
  }

  .dsp-controls-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .dsp-meter-row {
    display: flex;
    align-items: center;
    margin-bottom: 12px;
  }

  .gate-status-pill {
    font-size: 10px;
    font-weight: 800;
    padding: 3px 8px;
    border-radius: var(--radius-xs, 3px);
    letter-spacing: 0.5px;
    transition: all var(--transition-fast);
  }

  .gate-status-pill.open {
    background: var(--color-accent-success-subtle);
    color: var(--color-accent-success, #00E599);
    border: 1px solid var(--color-accent-success, #00E599);
  }

  .gate-status-pill.closed {
    background: var(--color-surface-2, #171B22);
    color: var(--color-text-muted, #64748B);
    border: 1px solid var(--color-border-default, #232834);
  }

  .compressor-gr-pill {
    font-size: 10px;
    font-weight: 800;
    padding: 3px 8px;
    border-radius: var(--radius-xs, 3px);
    background: var(--color-surface-2, #171B22);
    color: var(--color-text-muted, #64748B);
    border: 1px solid var(--color-border-default, #232834);
    font-family: var(--font-mono);
    font-variant-numeric: var(--tabular-nums);
    transition: all var(--transition-fast);
  }

  .compressor-gr-pill.active {
    background: var(--color-accent-info-subtle);
    color: var(--color-accent-info, #00B4D8);
    border: 1px solid var(--color-accent-info, #00B4D8);
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
</style>
