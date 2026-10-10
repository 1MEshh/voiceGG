<script lang="ts">
  import type { ChannelId, SystemStatus, ActiveStream } from '../types';
  import { CHANNELS } from './theme';
  import { Volume2, VolumeX, SlidersHorizontal, Gamepad2, MessageSquare, Music, Headphones, Mic, MoreVertical } from '@lucide/svelte';

  export let status: SystemStatus;
  export let onSetVolume: (channel: ChannelId, volume: number) => void;
  export let onSetMute: (channel: ChannelId, muted: boolean) => void;
  export let onSetChatMix: (value: number) => void;
  export let onRouteApp: (binary: string, target: ChannelId) => void;
  export let onOpenPresetBrowser: (channel: ChannelId) => void;
  export let onSetDevice: (type: 'sink' | 'source', deviceName: string) => void;

  const channelList: ChannelId[] = ['master', 'game', 'chat', 'media', 'aux', 'mic'];
  const channelIcons: Record<ChannelId, any> = { master: SlidersHorizontal, game: Gamepad2, chat: MessageSquare, media: Music, aux: Headphones, mic: Mic };

  let isDraggingAny = false;
  let dragOverChannel: ChannelId | null = null;
  let dragCounters: Record<string, number> = {};

  function handleDragStart(e: DragEvent, stream: ActiveStream) {
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('application/json', JSON.stringify(stream));
      isDraggingAny = true;
    }
  }

  function handleDragEnd() {
    isDraggingAny = false;
    dragOverChannel = null;
    dragCounters = {};
  }

  function handleDragEnter(e: DragEvent, channel: ChannelId) {
    e.preventDefault();
    if (channel === 'mic') return;
    dragCounters[channel] = (dragCounters[channel] || 0) + 1;
    dragOverChannel = channel;
  }

  function handleDragOver(e: DragEvent, channel: ChannelId) {
    e.preventDefault();
    if (channel === 'mic') return;
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dragOverChannel = channel;
  }

  function handleDragLeave(e: DragEvent, channel: ChannelId) {
    e.preventDefault();
    dragCounters[channel] = Math.max(0, (dragCounters[channel] || 0) - 1);
    if (dragCounters[channel] === 0 && dragOverChannel === channel) {
      dragOverChannel = null;
    }
  }

  function handleDrop(e: DragEvent, targetChannel: ChannelId) {
    e.preventDefault();
    isDraggingAny = false;
    dragOverChannel = null;
    dragCounters = {};
    if (targetChannel === 'mic') return;
    if (e.dataTransfer) {
      const data = e.dataTransfer.getData('application/json');
      if (data) {
        try {
          const stream = JSON.parse(data) as ActiveStream;
          onRouteApp(stream.binary_name, targetChannel);
        } catch (err) {
          console.error("Failed to parse dragged stream", err);
        }
      }
    }
  }
</script>

<div class="mixer-view">
  <div class="device-header">
    <div class="device-col">
      <span class="label">PLAYBACK DEVICE</span>
      <select class="master-select" value={status.config.preferred_output_device || 'default'} on:change={(e) => onSetDevice('sink', e.currentTarget.value)}>
        <option value="default">Default Headphone Sink</option>
        {#each (status.devices || []).filter(d => d.device_type.toLowerCase() === 'sink') as dev}
          <option value={dev.name}>{dev.description || dev.name}</option>
        {/each}
      </select>
    </div>
    <div class="device-col">
      <span class="label">MICROPHONE</span>
      <select class="master-select" value={status.config.preferred_input_device || 'default'} on:change={(e) => onSetDevice('source', e.currentTarget.value)}>
        <option value="default">Default System Mic</option>
        {#each (status.devices || []).filter(d => d.device_type.toLowerCase() === 'source') as dev}
          <option value={dev.name}>{dev.description || dev.name}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="channel-strips">
    {#each channelList as ch}
      {@const info = CHANNELS[ch]}
      {@const vol = status.config.volumes[ch] ?? 100}
      {@const isMuted = status.config.muted[ch] ?? false}
      {@const presetId = status.config.active_presets[ch] ?? 'Default'}
      {@const streams = (status.streams || []).filter(s => ch === 'master' ? (!s.current_channel || s.current_channel === 'master') : (s.current_channel === ch))}

      <div
        class="strip"
        class:drag-over={dragOverChannel === ch}
        class:is-dimmed={isDraggingAny && dragOverChannel !== ch && ch !== 'mic'}
        class:is-eligible={isDraggingAny && ch !== 'mic'}
        role="region"
        on:dragenter={(e) => handleDragEnter(e, ch)}
        on:dragover={(e) => handleDragOver(e, ch)}
        on:dragleave={(e) => handleDragLeave(e, ch)}
        on:drop={(e) => handleDrop(e, ch)}
      >
        <div class="strip-header">
          <svelte:component this={channelIcons[ch]} size={14} color={info.defaultColor} />
          <span class="channel-name">{info.label}</span>
        </div>

        <button class="preset-btn" on:click={() => onOpenPresetBrowser(ch)}>
          {presetId.replace(/^game_|^chat_|^media_|^mic_/, '').replace('_', ' ').toUpperCase()}
        </button>

        <div class="fader-container">
          <div class="fader-track">
            <div class="fader-fill" style="height: {isMuted ? '0%' : vol}%; background-color: {info.defaultColor};"></div>
          </div>
          <input type="range" class="vertical-slider" min="0" max="100" value={vol} on:input={(e) => onSetVolume(ch, Number(e.currentTarget.value))} />
        </div>

        <div class="vol-text">{vol}%</div>

        <button class="mute-btn" class:muted={isMuted} on:click={() => onSetMute(ch, !isMuted)}>
          {#if isMuted} <VolumeX size={14} /> {:else} <Volume2 size={14} /> {/if}
        </button>

        {#if ch !== 'mic'}
          <div class="apps-box">
            {#each streams as stream}
              <div
                class="app-chip"
                role="button"
                tabindex="0"
                draggable="true"
                on:dragstart={(e) => handleDragStart(e, stream)}
                on:dragend={handleDragEnd}
              >
                <span class="chip-color" style="background: {info.defaultColor};"></span>
                <span class="chip-name">{stream.binary_name || stream.app_name}</span>
              </div>
            {/each}
            {#if streams.length === 0}
              <div class="empty-drop-hint" class:highlight={dragOverChannel === ch}>
                {isDraggingAny ? 'DROP HERE' : 'NO APPS'}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="chatmix-bar">
    <span class="chatmix-label">GAME ({100 - Math.max(0, status.config.chatmix)}%)</span>
    <input type="range" min="-100" max="100" value={status.config.chatmix} on:input={(e) => onSetChatMix(Number(e.currentTarget.value))} class="chatmix-slider" />
    <span class="chatmix-label chat">CHAT ({100 + Math.min(0, status.config.chatmix)}%)</span>
  </div>
</div>

<style>
  .mixer-view { display: flex; flex-direction: column; height: 100%; background: #0d1117; padding: 20px; gap: 20px; font-family: 'Inter', sans-serif; overflow-y: auto; }
  .device-header { display: flex; gap: 20px; background: #161b22; padding: 12px 20px; border: 1px solid #30363d; border-radius: 4px; }
  .device-col { display: flex; flex-direction: column; gap: 4px; flex: 1; }
  .label { font-size: 10px; font-weight: 700; color: #8b949e; letter-spacing: 0.5px; }
  .master-select { background: #0d1117; color: #c9d1d9; border: 1px solid #30363d; padding: 6px; font-size: 12px; border-radius: 2px; outline: none; }
  
  .channel-strips { display: grid; grid-template-columns: repeat(6, 1fr); gap: 12px; flex: 1; min-height: 400px; }
  .strip {
    background: var(--color-surface-1, #11151F);
    border: 1px solid var(--color-border-default, #262E40);
    border-radius: var(--radius-md, 6px);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 16px 12px;
    transition: border-color 0.2s ease, opacity 0.2s ease, transform 0.2s ease, box-shadow 0.2s ease;
  }
  .strip.is-dimmed { opacity: 0.55; }
  .strip.is-eligible { border-style: dashed; }
  .strip.drag-over {
    border-style: solid;
    border-color: #38BDF8;
    background: var(--color-surface-hover, #252D40);
    box-shadow: 0 0 18px rgba(56, 189, 248, 0.25);
    transform: translateY(-3px);
  }
  .empty-drop-hint {
    padding: 10px 4px;
    border: 1px dashed var(--color-border-default, #262E40);
    border-radius: 4px;
    font-size: 9px;
    font-weight: 700;
    color: var(--color-text-muted, #64748B);
    text-align: center;
    letter-spacing: 0.5px;
    margin-top: 6px;
    transition: all 0.15s ease;
  }
  .empty-drop-hint.highlight {
    border-color: #38BDF8;
    color: #38BDF8;
    background: rgba(56, 189, 248, 0.1);
  }
  
  .strip-header { display: flex; align-items: center; gap: 6px; margin-bottom: 12px; }
  .channel-name { font-size: 12px; font-weight: 700; color: #c9d1d9; }
  
  .preset-btn { width: 100%; background: #0d1117; border: 1px solid #30363d; color: #8b949e; font-size: 10px; font-weight: 600; padding: 4px; border-radius: 2px; cursor: pointer; margin-bottom: 16px; }
  .preset-btn:hover { background: #1f242c; color: #c9d1d9; }
  
  .fader-container { position: relative; height: 180px; width: 24px; display: flex; justify-content: center; align-items: center; margin-bottom: 12px; }
  .fader-track { position: absolute; bottom: 0; width: 8px; height: 100%; background: #0d1117; border-radius: 4px; border: 1px solid #30363d; overflow: hidden; display: flex; align-items: flex-end; }
  .fader-fill { width: 100%; transition: height 0.1s ease-out; }
  
  .vertical-slider {
    position: absolute;
    width: 180px; /* Equal to container height */
    height: 24px;
    transform: rotate(-90deg);
    appearance: none;
    background: transparent;
    outline: none;
    cursor: pointer;
    z-index: 2;
  }
  
  .vertical-slider::-webkit-slider-thumb {
    appearance: none;
    width: 24px;
    height: 12px;
    background: #e2e8f0;
    border-radius: 2px;
    border: 1px solid #000;
    cursor: grab;
    box-shadow: 0 2px 4px rgba(0,0,0,0.5);
  }
  .vertical-slider::-webkit-slider-thumb:active { cursor: grabbing; }
  
  .vol-text { font-size: 12px; font-weight: 600; color: #c9d1d9; font-family: monospace; margin-bottom: 8px; }
  
  .mute-btn { width: 32px; height: 32px; background: #0d1117; border: 1px solid #30363d; color: #8b949e; display: flex; justify-content: center; align-items: center; border-radius: 2px; cursor: pointer; margin-bottom: 16px; }
  .mute-btn:hover { background: #1f242c; color: #c9d1d9; }
  .mute-btn.muted { background: #3b1818; border-color: #f85149; color: #f85149; }
  
  .apps-box { width: 100%; flex: 1; background: #0d1117; border: 1px solid #30363d; padding: 4px; display: flex; flex-direction: column; gap: 4px; overflow-y: auto; border-radius: 2px; }
  .app-chip { display: flex; align-items: center; gap: 6px; background: #161b22; padding: 4px; border-radius: 2px; font-size: 10px; color: #c9d1d9; cursor: grab; border: 1px solid transparent; }
  .app-chip:hover { border-color: #30363d; }
  .app-chip:active { cursor: grabbing; }
  .chip-color { width: 4px; height: 4px; border-radius: 50%; }
  
  .chatmix-bar { display: flex; align-items: center; gap: 16px; background: #161b22; padding: 12px 20px; border: 1px solid #30363d; border-radius: 4px; }
  .chatmix-label { font-size: 11px; font-weight: 700; color: #3fb950; font-family: monospace; }
  .chatmix-label.chat { color: #58a6ff; }
  .chatmix-slider { flex: 1; height: 4px; background: #0d1117; border: 1px solid #30363d; outline: none; appearance: none; border-radius: 2px; }
  .chatmix-slider::-webkit-slider-thumb { appearance: none; width: 12px; height: 16px; background: #8b949e; border-radius: 1px; cursor: pointer; }
</style>
