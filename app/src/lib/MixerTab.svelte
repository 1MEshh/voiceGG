<script lang="ts">
  import type { ChannelId, SystemStatus, ActiveStream, AudioDevice } from '../types';
  import { CHANNELS } from './theme';
  import { Volume2, VolumeX, SlidersHorizontal, Gamepad2, MessageSquare, Music, Headphones, Mic, MoreVertical } from '@lucide/svelte';

  export let status: SystemStatus;
  export let onSetVolume: (channel: ChannelId, volume: number) => void;
  export let onSetMute: (channel: ChannelId, muted: boolean) => void;
  export let onSetChatMix: (value: number) => void;
  export let onRouteApp: (binary: string, target: ChannelId) => void;
  export let onOpenPresetBrowser: (channel: ChannelId) => void;

  const channelList: ChannelId[] = ['master', 'game', 'chat', 'media', 'aux', 'mic'];

  const channelIcons: Record<ChannelId, any> = {
    master: SlidersHorizontal,
    game: Gamepad2,
    chat: MessageSquare,
    media: Music,
    aux: Headphones,
    mic: Mic,
  };

  let draggedApp: ActiveStream | null = null;
  let dragOverChannel: ChannelId | null = null;

  function handleDragStart(stream: ActiveStream) {
    draggedApp = stream;
  }

  function handleDragOver(e: DragEvent, channel: ChannelId) {
    e.preventDefault();
    dragOverChannel = channel;
  }

  function handleDragLeave() {
    dragOverChannel = null;
  }

  function handleDrop(e: DragEvent, targetChannel: ChannelId) {
    e.preventDefault();
    if (draggedApp) {
      onRouteApp(draggedApp.binary_name, targetChannel);
      draggedApp = null;
    }
    dragOverChannel = null;
  }

  function getStreamsForChannel(channel: ChannelId): ActiveStream[] {
    if (channel === 'master') {
      // Unassigned streams
      return status.streams.filter(s => !s.current_channel || s.current_channel === 'master');
    }
    return status.streams.filter(s => s.current_channel === channel);
  }
</script>

<div class="mixer-view">
  <div class="channel-strips">
    {#each channelList as ch}
      {@const info = CHANNELS[ch]}
      {@const vol = status.config.volumes[ch] ?? 100}
      {@const isMuted = status.config.muted[ch] ?? false}
      {@const presetId = status.config.active_presets[ch] ?? 'Default'}
      {@const streams = getStreamsForChannel(ch)}

      <div
        class="strip"
        class:drag-over={dragOverChannel === ch}
        role="region"
        aria-label="{info.label} channel strip"
        on:dragover={(e) => handleDragOver(e, ch)}
        on:dragleave={handleDragLeave}
        on:drop={(e) => handleDrop(e, ch)}
      >
        <!-- Header -->
        <div class="strip-header">
          <div class="header-left">
            <span class="channel-indicator" style="background-color: {info.defaultColor};"></span>
            <svelte:component this={channelIcons[ch]} size={16} color={info.defaultColor} />
            <span class="channel-name">{info.label}</span>
          </div>
          <button class="menu-btn" title="Channel Settings">
            <MoreVertical size={14} />
          </button>
        </div>

        <!-- Preset Pill -->
        <button
          class="preset-pill"
          on:click={() => onOpenPresetBrowser(ch)}
          title="Change Equalizer Preset"
        >
          <span class="preset-label">{presetId.replace(/^game_|^chat_|^media_|^mic_/, '').replace('_', ' ').toUpperCase()}</span>
        </button>

        <!-- Device Selector -->
        <div class="device-row">
          <select class="device-select">
            <option value="default">Default Headphone Sink</option>
            {#each status.devices.filter(d => ch === 'mic' ? d.device_type === 'Source' : d.device_type === 'Sink') as dev}
              <option value={dev.name}>{dev.description}</option>
            {/each}
          </select>
        </div>

        <!-- Fader & Level Meter -->
        <div class="fader-section">
          <div class="meter-track">
            <!-- Simulated active signal level with peak -->
            <div
              class="meter-fill"
              style="height: {isMuted ? '0%' : Math.min(100, vol * 0.75)}%; background: linear-gradient(to top, #22c55e 60%, #eab308 85%, #ef4444 100%);"
            ></div>
          </div>

          <input
            type="range"
            orient="vertical"
            class="vertical-slider"
            min="0"
            max="150"
            value={vol}
            on:input={(e) => onSetVolume(ch, Number(e.currentTarget.value))}
            title="Double-click to reset volume"
            on:dblclick={() => onSetVolume(ch, 100)}
          />
        </div>

        <!-- Volume readout -->
        <div class="volume-readout">
          <span>{vol}%</span>
        </div>

        <!-- Mute Button -->
        <button
          class="mute-btn"
          class:muted={isMuted}
          on:click={() => onSetMute(ch, !isMuted)}
          title={isMuted ? 'Unmute' : 'Mute'}
        >
          {#if isMuted}
            <VolumeX size={16} />
          {:else}
            <Volume2 size={16} />
          {/if}
        </button>

        <!-- App Routing Chips Box -->
        <div class="apps-box">
          <div class="apps-box-header">
            <span>{ch === 'master' ? 'ROUTED APPS' : 'APPS'} ({streams.length})</span>
          </div>

          <div class="chips-container">
            {#each streams as stream (stream.id)}
              <div
                class="app-chip"
                role="button"
                tabindex="0"
                draggable="true"
                on:dragstart={() => handleDragStart(stream)}
                title="Drag to move this application to another channel"
              >
                <span class="chip-dot" style="background-color: {info.defaultColor};"></span>
                <span class="chip-name">{stream.binary_name || stream.app_name}</span>
              </div>
            {:else}
              <div class="empty-apps-hint">
                <span>Drop app here</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    {/each}
  </div>

  <!-- ChatMix Section -->
  <div class="chatmix-bar">
    <div class="chatmix-header">
      <Gamepad2 size={16} color="#22c55e" />
      <span class="chatmix-title">CHATMIX BALANCE</span>
      <MessageSquare size={16} color="#06b6d4" />
    </div>

    <div class="chatmix-slider-row">
      <span class="chatmix-side-label game-label">GAME ({100 - Math.max(0, status.config.chatmix)}%)</span>

      <input
        type="range"
        min="-100"
        max="100"
        value={status.config.chatmix}
        on:input={(e) => onSetChatMix(Number(e.currentTarget.value))}
        class="chatmix-slider"
      />

      <span class="chatmix-side-label chat-label">CHAT ({100 + Math.min(0, status.config.chatmix)}%)</span>
    </div>

    <button
      class="chatmix-reset-btn"
      on:click={() => onSetChatMix(0)}
      title="Reset ChatMix balance to center"
    >
      CENTER (0)
    </button>
  </div>
</div>

<style>
  .mixer-view {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 52px);
    background-color: #0d1117;
    padding: 16px;
    gap: 16px;
    overflow-y: auto;
    user-select: none;
  }

  .channel-strips {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 12px;
    flex: 1;
    min-height: 480px;
  }

  .strip {
    background: #161b22;
    border: 1px solid #232b36;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 12px 10px;
    transition: all 0.15s ease;
  }

  .strip.drag-over {
    border-color: #3b82f6;
    background: #1c2636;
  }

  .strip-header {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .channel-indicator {
    width: 4px;
    height: 12px;
    border-radius: 2px;
  }

  .channel-name {
    font-size: 13px;
    font-weight: 700;
    color: #e2e8f0;
  }

  .menu-btn {
    background: transparent;
    border: none;
    color: #64748b;
    cursor: pointer;
    padding: 2px;
    border-radius: 4px;
  }

  .menu-btn:hover {
    color: #cbd5e1;
    background: #242c38;
  }

  .preset-pill {
    width: 100%;
    padding: 5px 8px;
    background: #1f2733;
    border: 1px solid #2d3846;
    border-radius: 5px;
    color: #cbd5e1;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 8px;
    transition: all 0.15s ease;
  }

  .preset-pill:hover {
    background: #283344;
    border-color: #3d4c60;
  }

  .device-row {
    width: 100%;
    margin-bottom: 12px;
  }

  .device-select {
    width: 100%;
    background: #13171d;
    border: 1px solid #232a35;
    border-radius: 5px;
    color: #94a3b8;
    font-size: 11px;
    padding: 4px 6px;
    outline: none;
    cursor: pointer;
  }

  .fader-section {
    position: relative;
    width: 38px;
    height: 190px;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 10px;
  }

  .meter-track {
    position: absolute;
    width: 10px;
    height: 100%;
    background: #0d1117;
    border-radius: 4px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
  }

  .meter-fill {
    width: 100%;
    transition: height 0.08s ease-out;
  }

  .vertical-slider {
    writing-mode: vertical-lr;
    direction: rtl;
    width: 32px;
    height: 100%;
    appearance: none;
    background: transparent;
    cursor: pointer;
    z-index: 2;
  }

  .vertical-slider::-webkit-slider-thumb {
    appearance: none;
    width: 24px;
    height: 12px;
    background: #e2e8f0;
    border-radius: 3px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.6);
    border: 1px solid #475569;
  }

  .volume-readout {
    font-family: monospace;
    font-size: 12px;
    font-weight: 700;
    color: #e2e8f0;
    margin-bottom: 8px;
  }

  .mute-btn {
    width: 34px;
    height: 34px;
    border-radius: 6px;
    border: 1px solid #2d3846;
    background: #1c2430;
    color: #94a3b8;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    margin-bottom: 12px;
    transition: all 0.15s ease;
  }

  .mute-btn:hover {
    background: #253040;
    color: #f1f5f9;
  }

  .mute-btn.muted {
    background: #3b1818;
    border-color: #ef444488;
    color: #ef4444;
  }

  .apps-box {
    width: 100%;
    flex: 1;
    background: #11151c;
    border: 1px solid #202732;
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    padding: 6px;
    overflow: hidden;
  }

  .apps-box-header {
    font-size: 10px;
    font-weight: 700;
    color: #64748b;
    margin-bottom: 6px;
    letter-spacing: 0.5px;
  }

  .chips-container {
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    flex: 1;
  }

  .app-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 7px;
    background: #1c232d;
    border: 1px solid #283340;
    border-radius: 4px;
    font-size: 11px;
    color: #cbd5e1;
    cursor: grab;
    transition: all 0.15s ease;
  }

  .app-chip:active {
    cursor: grabbing;
  }

  .app-chip:hover {
    background: #242e3b;
    border-color: #38475a;
  }

  .chip-dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
  }

  .chip-name {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty-apps-hint {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    font-size: 10px;
    color: #475569;
    font-style: italic;
  }

  .chatmix-bar {
    background: #161b22;
    border: 1px solid #232b36;
    border-radius: 8px;
    padding: 12px 20px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
  }

  .chatmix-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .chatmix-title {
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: #e2e8f0;
  }

  .chatmix-slider-row {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 14px;
  }

  .chatmix-side-label {
    font-size: 11px;
    font-weight: 700;
    font-family: monospace;
  }

  .game-label {
    color: #22c55e;
  }

  .chat-label {
    color: #06b6d4;
  }

  .chatmix-slider {
    flex: 1;
    appearance: none;
    height: 6px;
    background: #242c38;
    border-radius: 3px;
    outline: none;
    cursor: pointer;
  }

  .chatmix-slider::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #38bdf8;
    cursor: pointer;
    box-shadow: 0 0 6px rgba(56, 189, 248, 0.6);
  }

  .chatmix-reset-btn {
    padding: 5px 10px;
    background: #202732;
    border: 1px solid #2d3846;
    border-radius: 5px;
    color: #94a3b8;
    font-size: 10px;
    font-weight: 700;
    cursor: pointer;
  }

  .chatmix-reset-btn:hover {
    background: #283344;
    color: #f1f5f9;
  }
</style>
