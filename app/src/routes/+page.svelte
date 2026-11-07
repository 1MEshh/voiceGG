<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { ChannelId, SystemStatus, Preset, ChannelMeter } from '../types';
  import TopNav from '../lib/TopNav.svelte';
  import MixerTab from '../lib/MixerTab.svelte';
  import ChannelTab from '../lib/ChannelTab.svelte';
  import MicTab from '../lib/MicTab.svelte';
  import PresetBrowserModal from '../lib/PresetBrowserModal.svelte';
  import SettingsModal from '../lib/SettingsModal.svelte';
  import { t, setLanguage } from '../lib/i18n';

  let activeTab: 'mixer' | ChannelId = 'mixer';
  let isPresetBrowserOpen = false;
  let isSettingsOpen = false;
  let streamerMode = false;
  let browserTargetChannel: ChannelId = 'game';
  let languageInitialized = false;

  let isQuitting = false;
  let statusTimer: ReturnType<typeof setInterval> | null = null;
  let meterTimer: ReturnType<typeof setInterval> | null = null;
  let meterInFlight = false;
  let statusLoaded = false;

  let lastUserActionAt = 0;
  function markUserInteraction() {
    lastUserActionAt = Date.now();
  }

  // Initial fallback status
  let status: SystemStatus = {
    config: {
      language: 'en',
      active_presets: {
        master: 'game_flat',
        game: 'game_cs2',
        chat: 'chat_clear_voice',
        media: 'media_music_bass',
        aux: 'game_flat',
        mic: 'mic_broadcast',
      },
      volumes: {
        master: 100,
        game: 100,
        chat: 100,
        media: 100,
        aux: 100,
        mic: 100,
      },
      muted: {
        master: false,
        game: false,
        chat: false,
        media: false,
        aux: false,
        mic: false,
      },
      chatmix: 0,
      routing_rules: [],
      auto_game_detection: true,
      custom_presets: {},
    },
    devices: [],
    streams: [],
    active_game: null,
    capabilities: {
      rnnoise: false,
      gate: false,
      compressor: false,
      biquad_eq: true,
    },
    mic_monitor_enabled: false,
    mic_monitor_volume: 50,
  };

  let meters: Record<ChannelId, ChannelMeter> = {
    master: { peak_dbfs: -100, rms_dbfs: -100 },
    game: { peak_dbfs: -100, rms_dbfs: -100 },
    chat: { peak_dbfs: -100, rms_dbfs: -100 },
    media: { peak_dbfs: -100, rms_dbfs: -100 },
    aux: { peak_dbfs: -100, rms_dbfs: -100 },
    mic: { peak_dbfs: -100, rms_dbfs: -100 },
  };

  let builtinPresets: Preset[] = [];

  $: allPresets = (() => {
    const customMap = status?.config?.custom_presets || {};
    const merged = builtinPresets.map((bp: any) => customMap[bp.id] ? customMap[bp.id] : bp);
    const builtinIds = new Set(builtinPresets.map((bp: any) => bp.id));
    const extraCustom = Object.values(customMap).filter((cp: any) => !builtinIds.has(cp.id));
    return [...merged, ...extraCustom];
  })();

  async function fetchStatus(force = false) {
    if (isQuitting) return;
    if (!force && Date.now() - lastUserActionAt < 1200) {
      return;
    }
    try {
      const res = await invoke<SystemStatus>('get_status');
      if (isQuitting) return;
      if (!force && Date.now() - lastUserActionAt < 1200) {
        return;
      }
      status = res;
      statusLoaded = true;
      streamerMode = Boolean(res.config.streamer_mode);
      if (!languageInitialized && res.config.language) {
        setLanguage(res.config.language === 'ar' ? 'ar' : 'en');
        languageInitialized = true;
      }
    } catch (e) {
      console.warn('Could not fetch status from daemon:', e);
      statusLoaded = true;
    }
  }

  async function fetchMeters() {
    if (meterInFlight || isQuitting || (typeof document !== 'undefined' && document.hidden)) {
      return;
    }
    meterInFlight = true;
    try {
      const res = await invoke<Record<ChannelId, ChannelMeter>>('get_meters');
      if (res && !isQuitting) {
        meters = res;
      }
    } catch {
      // Ignore background meter polling errors
    } finally {
      meterInFlight = false;
    }
  }

  async function fetchPresets() {
    try {
      const presets = await invoke<Preset[]>('get_builtin_presets');
      builtinPresets = presets;
    } catch (e) {
      console.warn('Could not fetch presets:', e);
    }
  }

  async function handleSetVolume(channel: ChannelId, volume: number) {
    markUserInteraction();
    status.config.volumes[channel] = volume;
    status = { ...status };
    try {
      await invoke('set_volume', { channel, volume });
    } catch (e) {
      console.error('Failed to set volume:', e);
    }
  }

  async function handleSetMute(channel: ChannelId, muted: boolean) {
    markUserInteraction();
    status.config.muted[channel] = muted;
    status = { ...status };
    try {
      await invoke('set_mute', { channel, muted });
    } catch (e) {
      console.error('Failed to set mute:', e);
    }
  }

  async function handleSetChatMix(value: number) {
    markUserInteraction();
    status.config.chatmix = value;
    status = { ...status };
    try {
      await invoke('set_chatmix', { value });
    } catch (e) {
      console.error('Failed to set chatmix:', e);
    }
  }

  async function handleRouteApp(binaryName: string, targetChannel: ChannelId) {
    markUserInteraction();
    const lowerTarget = binaryName.toLowerCase();
    status.streams = (status.streams || []).map((s) => {
      if (
        s.binary_name.toLowerCase() === lowerTarget ||
        s.app_name.toLowerCase() === lowerTarget
      ) {
        return { ...s, current_channel: targetChannel };
      }
      return s;
    });
    status = { ...status };
    try {
      await invoke('route_app', { binaryName, targetChannel });
    } catch (e) {
      console.error('Failed to route app:', e);
    }
  }

  async function handleSetAppVolume(binaryName: string, volume: number) {
    markUserInteraction();
    if (!status.config.app_volumes) {
      status.config.app_volumes = {};
    }
    status.config.app_volumes[binaryName] = volume;
    status = { ...status };
    try {
      await invoke('set_app_volume', { binaryName, volume });
    } catch (e) {
      console.error('Failed to set app volume:', e);
    }
  }

  async function handleToggleStreamer() {
    markUserInteraction();
    streamerMode = !streamerMode;
    status.config.streamer_mode = streamerMode;
    status = { ...status };
    try {
      await invoke('set_streamer_mode', { enabled: streamerMode });
    } catch (e) {
      console.error('Failed to set streamer mode:', e);
    }
  }

  async function handleSetPersonalMix(channel: ChannelId, enabled: boolean) {
    markUserInteraction();
    if (!status.config.personal_mix) status.config.personal_mix = {};
    status.config.personal_mix[channel] = enabled;
    status = { ...status };
    try {
      await invoke('set_personal_mix', { channel, enabled });
    } catch (e) {
      console.error('Failed to set personal mix:', e);
    }
  }

  async function handleSetStreamMix(channel: ChannelId, enabled: boolean) {
    markUserInteraction();
    if (!status.config.stream_mix) status.config.stream_mix = {};
    status.config.stream_mix[channel] = enabled;
    status = { ...status };
    try {
      await invoke('set_stream_mix', { channel, enabled });
    } catch (e) {
      console.error('Failed to set stream mix:', e);
    }
  }

  async function handleApplyPreset(preset: Preset) {
    markUserInteraction();
    const ch = activeTab === 'mixer' ? browserTargetChannel : activeTab;
    status.config.active_presets[ch] = preset.id;
    status = { ...status };
    try {
      await invoke('set_preset', { channel: ch, presetId: preset.id });
    } catch (e) {
      console.error('Failed to apply preset:', e);
    }
    isPresetBrowserOpen = false;
  }

  async function handleSavePreset(channel: ChannelId, preset: Preset) {
    markUserInteraction();
    if (!status.config.custom_presets) {
      status.config.custom_presets = {};
    }
    status.config.custom_presets[preset.id] = JSON.parse(JSON.stringify(preset));
    status.config.active_presets[channel] = preset.id;
    status = { ...status };
    try {
      await invoke('save_preset', { channel, preset });
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  async function handleToggleFavorite(channel: ChannelId, presetId: string) {
    markUserInteraction();
    if (!status.config.favorite_presets) {
      status.config.favorite_presets = {};
    }
    const list = status.config.favorite_presets[channel] || [];
    const idx = list.indexOf(presetId);
    if (idx >= 0) list.splice(idx, 1);
    else list.push(presetId);
    status.config.favorite_presets[channel] = [...list];
    status = { ...status };
    try {
      await invoke('toggle_favorite_preset', { channel, presetId });
    } catch (e) {
      console.error('Failed to toggle favorite preset:', e);
    }
  }

  async function handleSelectPreset(channel: ChannelId, presetId: string) {
    markUserInteraction();
    status.config.active_presets[channel] = presetId;
    status = { ...status };
    try {
      await invoke('set_preset', { channel, presetId });
    } catch (e) {
      console.error('Failed to select preset:', e);
    }
  }

  async function handleResetPreset(channel: ChannelId, presetId: string) {
    markUserInteraction();
    if (status.config.custom_presets && status.config.custom_presets[presetId]) {
      delete status.config.custom_presets[presetId];
      status = { ...status };
    }
    try {
      await invoke('reset_preset', { channel, presetId });
    } catch (e) {
      console.warn('Backend reset_preset failed or not available, falling back:', e);
      const original = builtinPresets.find((bp) => bp.id === presetId);
      if (original) {
        await invoke('save_preset', { channel, preset: original });
      }
    }
    await fetchStatus(true);
  }

  async function handleDeleteCustomPreset(presetId: string) {
    markUserInteraction();
    if (status.config.custom_presets && status.config.custom_presets[presetId]) {
      delete status.config.custom_presets[presetId];
      status = { ...status };
    }
    try {
      await invoke('delete_custom_preset', { presetId });
    } catch (e) {
      console.error('Failed to delete custom preset:', e);
    }
    await fetchStatus(true);
  }

  async function handleSaveCustomPreset(channel: ChannelId, preset: Preset) {
    markUserInteraction();
    if (!status.config.custom_presets) {
      status.config.custom_presets = {};
    }
    status.config.custom_presets[preset.id] = JSON.parse(JSON.stringify(preset));
    status.config.active_presets[channel] = preset.id;
    status = { ...status };
    try {
      await invoke('save_custom_preset', { channel, preset });
    } catch (e) {
      console.error('Failed to save custom preset:', e);
    }
  }

  async function handleSetMicMonitor(enabled: boolean, volume: number) {
    markUserInteraction();
    status.mic_monitor_enabled = enabled;
    status.mic_monitor_volume = volume;
    status = { ...status };
    try {
      await invoke('set_mic_monitor', { enabled, volume });
    } catch (e) {
      console.error('Failed to set mic monitor:', e);
    }
  }

  async function handlePanicReset() {
    if (confirm($t('settings.confirm_panic'))) {
      markUserInteraction();
      try {
        await invoke('panic_reset');
        await fetchStatus(true);
      } catch (e) {
        console.error('Failed to panic reset:', e);
      }
    }
  }

  async function handleHideWindow() {
    try {
      await invoke('hide_window');
    } catch (e) {
      console.error('Failed to hide window:', e);
    }
  }

  async function handleQuitApp() {
    if (confirm($t('settings.confirm_quit'))) {
      isQuitting = true;
      if (statusTimer) {
        clearInterval(statusTimer);
        statusTimer = null;
      }
      if (meterTimer) {
        clearInterval(meterTimer);
        meterTimer = null;
      }
      try {
        await invoke('quit_app');
      } catch (e) {
        console.error('Failed to quit app:', e);
      }
    }
  }

  async function handleSetQuantum(quantum: number) {
    markUserInteraction();
    status.config.buffer_quantum = quantum;
    status = { ...status };
    try {
      await invoke('set_quantum', { quantum });
    } catch (e) {
      console.error('Failed to set quantum:', e);
    }
  }

  async function handleSetChannelColor(channel: string, hex: string) {
    markUserInteraction();
    if (!status.config.theme) {
      status.config.theme = {
        name: 'Sonar Dark',
        background_hex: '#0C0E12',
        surface_hex: '#12151B',
        text_primary_hex: '#F8FAFC',
        channels: {},
        meter_style: 'segmented',
        follow_pywal: false,
      };
    }
    if (!status.config.theme.channels) {
      status.config.theme.channels = {};
    }
    status.config.theme.channels[channel as ChannelId] = {
      accent_hex: hex,
      secondary_hex: hex,
    };
    status = { ...status };
    try {
      await invoke('set_channel_color', { channel, hex });
    } catch (e) {
      console.error('Failed to set channel color:', e);
    }
  }

  async function handleSetDevice(type: 'sink' | 'source', deviceName: string) {
    markUserInteraction();
    if (type === 'sink') {
      status.config.preferred_output_device = deviceName;
    } else {
      status.config.preferred_input_device = deviceName;
    }
    status = { ...status };
    try {
      await invoke('set_audio_device', { deviceType: type, deviceName });
      await fetchStatus(true);
    } catch (e) {
      console.error('Failed to set audio device:', e);
    }
  }

  function openPresetBrowser(channel: ChannelId) {
    browserTargetChannel = channel;
    isPresetBrowserOpen = true;
  }

  onMount(() => {
    fetchPresets();
    fetchStatus(true);
    statusTimer = setInterval(() => fetchStatus(false), 1500);
    meterTimer = setInterval(fetchMeters, 60);
    return () => {
      if (statusTimer) {
        clearInterval(statusTimer);
        statusTimer = null;
      }
      if (meterTimer) {
        clearInterval(meterTimer);
        meterTimer = null;
      }
    };
  });

  onDestroy(() => {
    if (statusTimer) {
      clearInterval(statusTimer);
      statusTimer = null;
    }
    if (meterTimer) {
      clearInterval(meterTimer);
      meterTimer = null;
    }
  });

  // Active preset object for current tab
  $: currentChannelId = (activeTab === 'mixer' ? 'game' : activeTab) as ChannelId;
  $: currentPresetId = status.config.active_presets[currentChannelId] || 'game_flat';
  $: currentPresetObj =
    allPresets.find(p => p.id === currentPresetId) ||
    allPresets.find(p => p.category === currentChannelId) ||
    allPresets[0];
</script>

<div class="voicegg-app">
  <TopNav
    {activeTab}
    activeGame={status.active_game}
    {streamerMode}
    onSelectTab={(tab) => activeTab = tab}
    onOpenSettings={() => isSettingsOpen = true}
    onToggleStreamer={handleToggleStreamer}
    onHideWindow={handleHideWindow}
    onQuitApp={handleQuitApp}
  />

  {#if !statusLoaded}
    <div class="cold-start-loading" role="status" aria-live="polite">
      <div class="sonar-spinner"></div>
      <span class="loading-label">Connecting to VoiceGG Engine...</span>
    </div>
  {:else}
    <main class="main-viewport">
      {#if activeTab === 'mixer'}
        <MixerTab
          {status}
          {allPresets}
          {streamerMode}
          micMonitorEnabled={status.mic_monitor_enabled}
          micMonitorVolume={status.mic_monitor_volume}
          {meters}
          onSetVolume={handleSetVolume}
          onSetMute={handleSetMute}
          onSetChatMix={handleSetChatMix}
          onRouteApp={handleRouteApp}
          onOpenPresetBrowser={openPresetBrowser}
          onSetDevice={handleSetDevice}
          onSetMicMonitor={handleSetMicMonitor}
          onSetAppVolume={handleSetAppVolume}
          onSetPersonalMix={handleSetPersonalMix}
          onSetStreamMix={handleSetStreamMix}
        />
      {:else if activeTab === 'mic'}
        {#if currentPresetObj}
          <MicTab
            currentPreset={currentPresetObj}
            {allPresets}
            {builtinPresets}
            devices={status.devices}
            preferredInputDevice={status.config.preferred_input_device}
            micMonitorEnabled={status.mic_monitor_enabled}
            micMonitorVolume={status.mic_monitor_volume}
            capabilities={status.capabilities}
            meter={meters['mic']}
            onChangePreset={(p) => handleSavePreset('mic', p)}
            onSelectPreset={(p) => handleSelectPreset('mic', p.id)}
            onResetPreset={handleResetPreset}
            onOpenBrowser={() => openPresetBrowser('mic')}
            onSetDevice={handleSetDevice}
            onSetMicMonitor={handleSetMicMonitor}
            onSaveCustomPreset={(ch, p) => handleSaveCustomPreset(ch, p)}
          />
        {/if}
      {:else}
        {#if currentPresetObj}
          <ChannelTab
            channel={activeTab as ChannelId}
            currentPreset={currentPresetObj}
            {allPresets}
            {builtinPresets}
            meter={meters[activeTab as ChannelId]}
            onApplyPreset={(p) => handleSavePreset(activeTab as ChannelId, p)}
            onSelectPreset={(p) => handleSelectPreset(activeTab as ChannelId, p.id)}
            onResetPreset={handleResetPreset}
            onOpenBrowser={() => openPresetBrowser(activeTab as ChannelId)}
            onSaveCustomPreset={(ch, p) => handleSaveCustomPreset(ch, p)}
          />
        {/if}
      {/if}
    </main>
  {/if}

  <!-- Preset Browser Modal -->
  <PresetBrowserModal
    isOpen={isPresetBrowserOpen}
    activeChannel={browserTargetChannel}
    {allPresets}
    activePresetId={status.config.active_presets[browserTargetChannel] || ''}
    onClose={() => isPresetBrowserOpen = false}
    onSelectPreset={handleApplyPreset}
    onDeletePreset={handleDeleteCustomPreset}
  />

  <!-- Settings Modal -->
  <SettingsModal
    isOpen={isSettingsOpen}
    {status}
    onClose={() => isSettingsOpen = false}
    onPanicReset={handlePanicReset}
    onToggleAutoDetect={(val) => {
      markUserInteraction();
      status.config.auto_game_detection = val;
    }}
    onSetQuantum={handleSetQuantum}
    onSetChannelColor={handleSetChannelColor}
  />
</div>

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    background-color: var(--color-bg-app, #0C0E12);
    color: var(--color-text-primary, #F8FAFC);
    font-family: var(--font-sans);
    overflow: hidden;
  }

  .voicegg-app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background-color: var(--color-bg-app, #0C0E12);
  }

  .cold-start-loading {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    background-color: var(--color-bg-app, #0C0E12);
  }

  .sonar-spinner {
    width: 36px;
    height: 36px;
    border: 3px solid rgba(255, 255, 255, 0.1);
    border-top-color: #FF5200;
    border-radius: 50%;
    animation: sonar-spin 0.8s linear infinite;
  }

  @keyframes sonar-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .loading-label {
    font-size: 13px;
    font-weight: 500;
    color: #94A3B8;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    font-family: var(--font-sans);
  }

  .main-viewport {
    flex: 1;
    overflow: hidden;
    background-color: var(--color-bg-app, #0C0E12);
  }
</style>
