<script lang="ts">
  import { onMount } from 'svelte';
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

  let allPresets: Preset[] = [];

  async function fetchStatus() {
    try {
      const res = await invoke<SystemStatus>('get_status');
      status = res;
      if (!languageInitialized && res.config.language) {
        setLanguage(res.config.language === 'ar' ? 'ar' : 'en');
        languageInitialized = true;
      }
    } catch (e) {
      console.warn('Could not fetch status from daemon:', e);
    }
  }

  async function fetchMeters() {
    try {
      const res = await invoke<Record<ChannelId, ChannelMeter>>('get_meters');
      if (res) {
        meters = res;
      }
    } catch {
      // Ignore background meter polling errors
    }
  }

  async function fetchPresets() {
    try {
      const presets = await invoke<Preset[]>('get_builtin_presets');
      allPresets = presets;
    } catch (e) {
      console.warn('Could not fetch presets:', e);
    }
  }

  async function handleSetVolume(channel: ChannelId, volume: number) {
    status.config.volumes[channel] = volume;
    try {
      await invoke('set_volume', { channel, volume });
    } catch (e) {
      console.error('Failed to set volume:', e);
    }
  }

  async function handleSetMute(channel: ChannelId, muted: boolean) {
    status.config.muted[channel] = muted;
    try {
      await invoke('set_mute', { channel, muted });
    } catch (e) {
      console.error('Failed to set mute:', e);
    }
  }

  async function handleSetChatMix(value: number) {
    status.config.chatmix = value;
    try {
      await invoke('set_chatmix', { value });
    } catch (e) {
      console.error('Failed to set chatmix:', e);
    }
  }

  async function handleRouteApp(binaryName: string, targetChannel: ChannelId) {
    // Instant optimistic update
    const stream = status.streams.find(
      s => s.binary_name.toLowerCase() === binaryName.toLowerCase() || s.app_name.toLowerCase() === binaryName.toLowerCase()
    );
    if (stream) {
      stream.current_channel = targetChannel;
      status = { ...status };
    }
    try {
      await invoke('route_app', { binaryName, targetChannel });
      await fetchStatus();
    } catch (e) {
      console.error('Failed to route app:', e);
    }
  }

  async function handleSetAppVolume(binaryName: string, volume: number) {
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
    streamerMode = !streamerMode;
    status.config.streamer_mode = streamerMode;
    try {
      await invoke('set_streamer_mode', { enabled: streamerMode });
    } catch (e) {
      console.error('Failed to set streamer mode:', e);
    }
  }

  async function handleSetPersonalMix(channel: ChannelId, enabled: boolean) {
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
    const ch = activeTab === 'mixer' ? browserTargetChannel : activeTab;
    status.config.active_presets[ch] = preset.id;
    try {
      await invoke('set_preset', { channel: ch, presetId: preset.id });
    } catch (e) {
      console.error('Failed to apply preset:', e);
    }
    isPresetBrowserOpen = false;
  }

  async function handleSavePreset(channel: ChannelId, preset: Preset) {
    try {
      await invoke('save_preset', { channel, preset });
    } catch (e) {
      console.error('Failed to save preset:', e);
    }
  }

  async function handleToggleFavorite(channel: ChannelId, presetId: string) {
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

  async function handleSaveCustomPreset(channel: ChannelId, preset: Preset) {
    if (!status.config.custom_presets) {
      status.config.custom_presets = {};
    }
    status.config.custom_presets[preset.id] = preset;
    status.config.active_presets[channel] = preset.id;
    allPresets = [...allPresets.filter((p) => p.id !== preset.id), preset];
    status = { ...status };
    try {
      await invoke('save_custom_preset', { channel, preset });
    } catch (e) {
      console.error('Failed to save custom preset:', e);
    }
  }

  async function handleSetMicMonitor(enabled: boolean, volume: number) {
    status.mic_monitor_enabled = enabled;
    status.mic_monitor_volume = volume;
    try {
      await invoke('set_mic_monitor', { enabled, volume });
    } catch (e) {
      console.error('Failed to set mic monitor:', e);
    }
  }

  async function handlePanicReset() {
    if (confirm($t('settings.confirm_panic'))) {
      try {
        await invoke('panic_reset');
        await fetchStatus();
      } catch (e) {
        console.error('Failed to panic reset:', e);
      }
    }
  }

  async function handleSetDevice(type: 'sink' | 'source', deviceName: string) {
    try {
      await invoke('set_audio_device', { deviceType: type, deviceName });
      await fetchStatus();
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
    fetchStatus();
    const interval = setInterval(fetchStatus, 1500);
    const meterInterval = setInterval(fetchMeters, 60);
    return () => {
      clearInterval(interval);
      clearInterval(meterInterval);
    };
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
    onPanicReset={handlePanicReset}
  />

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
          devices={status.devices}
          preferredInputDevice={status.config.preferred_input_device}
          micMonitorEnabled={status.mic_monitor_enabled}
          micMonitorVolume={status.mic_monitor_volume}
          capabilities={status.capabilities}
          meter={meters['mic']}
          onChangePreset={(p) => handleSavePreset('mic', p)}
          onOpenBrowser={() => openPresetBrowser('mic')}
          onSetDevice={handleSetDevice}
          onSetMicMonitor={handleSetMicMonitor}
        />
      {/if}
    {:else}
      {#if currentPresetObj}
        <ChannelTab
          channel={activeTab as ChannelId}
          currentPreset={currentPresetObj}
          {allPresets}
          favoritePresets={status.config.favorite_presets?.[activeTab as ChannelId] || []}
          meter={meters[activeTab as ChannelId]}
          onApplyPreset={(p) => handleSavePreset(activeTab as ChannelId, p)}
          onOpenBrowser={() => openPresetBrowser(activeTab as ChannelId)}
          onToggleFavorite={(presetId) => handleToggleFavorite(activeTab as ChannelId, presetId)}
          onSaveCustomPreset={(ch, p) => handleSaveCustomPreset(ch, p)}
          onChangeEq={(eq) => {
            currentPresetObj.eq = eq;
            handleSavePreset(activeTab as ChannelId, currentPresetObj);
          }}
        />
      {/if}
    {/if}
  </main>

  <!-- Preset Browser Modal -->
  <PresetBrowserModal
    isOpen={isPresetBrowserOpen}
    activeChannel={browserTargetChannel}
    {allPresets}
    activePresetId={status.config.active_presets[browserTargetChannel] || ''}
    onClose={() => isPresetBrowserOpen = false}
    onSelectPreset={handleApplyPreset}
  />

  <!-- Settings Modal -->
  <SettingsModal
    isOpen={isSettingsOpen}
    {status}
    onClose={() => isSettingsOpen = false}
    onPanicReset={handlePanicReset}
    onToggleAutoDetect={(val) => status.config.auto_game_detection = val}
  />
</div>

<style>
  :global(*) {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    background-color: var(--color-bg-app, #0B0E14);
    color: var(--color-text-primary, #F8FAFC);
    font-family: var(--font-sans);
    overflow: hidden;
  }

  .voicegg-app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
    background-color: var(--color-bg-app, #0B0E14);
  }

  .main-viewport {
    flex: 1;
    overflow: hidden;
    background-color: var(--color-bg-app, #0B0E14);
  }
</style>
