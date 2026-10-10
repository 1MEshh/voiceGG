<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import type { ChannelId, SystemStatus, Preset } from '../types';
  import TopNav from '../lib/TopNav.svelte';
  import MixerTab from '../lib/MixerTab.svelte';
  import ChannelTab from '../lib/ChannelTab.svelte';
  import MicTab from '../lib/MicTab.svelte';
  import PresetBrowserModal from '../lib/PresetBrowserModal.svelte';
  import SettingsModal from '../lib/SettingsModal.svelte';

  let activeTab: 'mixer' | ChannelId = 'mixer';
  let isPresetBrowserOpen = false;
  let isSettingsOpen = false;
  let streamerMode = false;
  let browserTargetChannel: ChannelId = 'game';

  // Initial fallback status
  let status: SystemStatus = {
    config: {
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
  };

  let allPresets: Preset[] = [];

  async function fetchStatus() {
    try {
      const res = await invoke<SystemStatus>('get_status');
      status = res;
    } catch (e) {
      console.warn('Could not fetch status from daemon:', e);
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
    try {
      await invoke('route_app', { binaryName, targetChannel });
      await fetchStatus();
    } catch (e) {
      console.error('Failed to route app:', e);
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

  async function handlePanicReset() {
    if (confirm('Are you sure you want to reset all VoiceGG audio routing to system defaults?')) {
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
    return () => clearInterval(interval);
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
    onToggleStreamer={() => streamerMode = !streamerMode}
    onPanicReset={handlePanicReset}
  />

  <main class="main-viewport">
    {#if activeTab === 'mixer'}
      <MixerTab
        {status}
        onSetVolume={handleSetVolume}
        onSetMute={handleSetMute}
        onSetChatMix={handleSetChatMix}
        onRouteApp={handleRouteApp}
        onOpenPresetBrowser={openPresetBrowser}
        onSetDevice={handleSetDevice}
      />
    {:else if activeTab === 'mic'}
      {#if currentPresetObj}
        <MicTab
          currentPreset={currentPresetObj}
          devices={status.devices}
          preferredInputDevice={status.config.preferred_input_device}
          onChangePreset={(p) => handleApplyPreset(p)}
          onSetDevice={handleSetDevice}
        />
      {/if}
    {:else}
      {#if currentPresetObj}
        <ChannelTab
          channel={activeTab as ChannelId}
          currentPreset={currentPresetObj}
          {allPresets}
          onApplyPreset={handleApplyPreset}
          onOpenBrowser={() => openPresetBrowser(activeTab as ChannelId)}
          onChangeEq={(eq) => {
            currentPresetObj.eq = eq;
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
    font-family: 'Inter', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
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
