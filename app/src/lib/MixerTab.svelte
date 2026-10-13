<script lang="ts">
  import type { ChannelId, SystemStatus, ActiveStream, ChannelMeter } from '../types';
  import { CHANNELS } from './theme';
  import { Volume2, VolumeX, SlidersHorizontal, Gamepad2, MessageSquare, Music, Headphones, Mic } from '@lucide/svelte';
  import { t } from './i18n';

  export let status: SystemStatus;
  export let micMonitorEnabled: boolean = false;
  export let micMonitorVolume: number = 50;
  export let meters: Record<ChannelId, ChannelMeter> | undefined = undefined;
  export let onSetVolume: (channel: ChannelId, volume: number) => void;
  export let onSetMute: (channel: ChannelId, muted: boolean) => void;
  export let onSetChatMix: (value: number) => void;
  export let onRouteApp: (binary: string, target: ChannelId) => void;
  export let onOpenPresetBrowser: (channel: ChannelId) => void;
  export let onSetDevice: (type: 'sink' | 'source', deviceName: string) => void;
  export let onSetMicMonitor: ((enabled: boolean, volume: number) => void) | undefined = undefined;

  function getMeterPercent(channel: ChannelId): number {
    const db = meters?.[channel]?.peak_dbfs ?? -100;
    if (db <= -60) return 0;
    if (db >= 0) return 100;
    return ((db + 60) / 60) * 100;
  }

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
      <span class="label">{$t('mixer.playback_device')}</span>
      <select
        class="master-select"
        aria-label={$t('mixer.playback_select_aria')}
        value={status.config.preferred_output_device || 'default'}
        on:change={(e) => onSetDevice('sink', e.currentTarget.value)}
      >
        <option value="default">{$t('mixer.default_sink')}</option>
        {#each (status.devices || []).filter(d => d.device_type.toLowerCase() === 'sink') as dev}
          <option value={dev.name}>{dev.description || dev.name}</option>
        {/each}
      </select>
    </div>
    <div class="device-col">
      <span class="label">{$t('mixer.microphone')}</span>
      <select
        class="master-select"
        aria-label={$t('mixer.mic_select_aria')}
        value={status.config.preferred_input_device || 'default'}
        on:change={(e) => onSetDevice('source', e.currentTarget.value)}
      >
        <option value="default">{$t('mixer.default_source')}</option>
        {#each (status.devices || []).filter(d => d.device_type.toLowerCase() === 'source') as dev}
          <option value={dev.name}>{dev.description || dev.name}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="channel-strips">
    {#each channelList as ch}
      {@const info = CHANNELS[ch]}
      {@const chLabel = $t(`channels.${ch}`)}
      {@const vol = status.config.volumes[ch] ?? 100}
      {@const isMuted = status.config.muted[ch] ?? false}
      {@const presetId = status.config.active_presets[ch] ?? 'Default'}
      {@const streams = (status.streams || []).filter(s => ch === 'master' ? (!s.current_channel || s.current_channel === 'master') : (s.current_channel === ch))}

      <div
        class="strip"
        style="--ch-color: {info.defaultColor};"
        class:drag-over={dragOverChannel === ch}
        class:is-dimmed={isDraggingAny && dragOverChannel !== ch && ch !== 'mic'}
        class:is-eligible={isDraggingAny && ch !== 'mic'}
        role="region"
        aria-label={$t('mixer.channel_strip_aria', { channel: chLabel })}
        on:dragenter={(e) => handleDragEnter(e, ch)}
        on:dragover={(e) => handleDragOver(e, ch)}
        on:dragleave={(e) => handleDragLeave(e, ch)}
        on:drop={(e) => handleDrop(e, ch)}
      >
        <div class="strip-header">
          <svelte:component this={channelIcons[ch]} size={14} color={info.defaultColor} />
          <span class="channel-name">{chLabel}</span>
        </div>

        <button
          class="preset-btn"
          aria-label={$t('mixer.preset_aria', { preset: presetId })}
          on:click={() => onOpenPresetBrowser(ch)}
        >
          {presetId.replace(/^game_|^chat_|^media_|^mic_/, '').replace('_', ' ').toUpperCase()}
        </button>

        <div class="fader-container">
          <div class="fader-track">
            <div
              class="fader-fill"
              style="height: {isMuted ? '0%' : vol}%; background-color: {info.defaultColor};"
            ></div>
          </div>
          <div
            class="meter-track"
            title={`${meters?.[ch]?.peak_dbfs !== undefined ? meters[ch].peak_dbfs.toFixed(1) + ' dBFS' : '-∞ dBFS'}`}
          >
            <div class="meter-fill" style="height: {getMeterPercent(ch)}%;"></div>
          </div>
          <input
            type="range"
            aria-label={$t('mixer.channel_vol_aria', { channel: chLabel })}
            aria-valuenow={vol}
            aria-valuemin="0"
            aria-valuemax="100"
            class="vertical-slider"
            min="0"
            max="100"
            value={vol}
            on:input={(e) => onSetVolume(ch, Number(e.currentTarget.value))}
          />
        </div>

        <div class="vol-text tabular-nums">{vol}%</div>

        <div class="strip-btn-row">
          <button
            class="mute-btn"
            class:muted={isMuted}
            on:click={() => onSetMute(ch, !isMuted)}
            aria-label={isMuted ? $t('mixer.unmute_ch', { channel: chLabel }) : $t('mixer.mute_ch', { channel: chLabel })}
            title={isMuted ? $t('mixer.unmute') : $t('mixer.mute')}
          >
            {#if isMuted} <VolumeX size={15} /> {:else} <Volume2 size={15} /> {/if}
          </button>
          {#if ch === 'mic'}
            <button
              class="strip-monitor-btn"
              class:active={micMonitorEnabled}
              on:click={() => onSetMicMonitor?.(!micMonitorEnabled, micMonitorVolume)}
              aria-label={micMonitorEnabled ? $t('mixer.disable_monitor') : $t('mixer.enable_monitor')}
              title={micMonitorEnabled ? $t('mixer.monitor_active') : $t('mixer.monitor_inactive')}
            >
              <Headphones size={15} />
            </button>
          {/if}
        </div>

        {#if ch !== 'mic'}
          <div class="apps-box">
            {#each streams as stream}
              <div
                class="app-chip"
                role="button"
                tabindex="0"
                draggable="true"
                aria-label={$t('mixer.route_app_aria', { name: stream.binary_name || stream.app_name })}
                on:dragstart={(e) => handleDragStart(e, stream)}
                on:dragend={handleDragEnd}
              >
                <span class="chip-color" style="background: {info.defaultColor};"></span>
                <span class="chip-name">{stream.binary_name || stream.app_name}</span>
              </div>
            {/each}
            {#if streams.length === 0}
              <div class="empty-drop-hint" class:highlight={dragOverChannel === ch}>
                {isDraggingAny ? $t('mixer.drop_here') : $t('mixer.no_apps')}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <div class="chatmix-bar">
    <span class="chatmix-label game tabular-nums">{$t('mixer.game')} ({100 - Math.max(0, status.config.chatmix)}%)</span>
    <input
      type="range"
      aria-label={$t('mixer.chatmix')}
      aria-valuenow={status.config.chatmix}
      aria-valuemin="-100"
      aria-valuemax="100"
      min="-100"
      max="100"
      value={status.config.chatmix}
      on:input={(e) => onSetChatMix(Number(e.currentTarget.value))}
      class="chatmix-slider"
    />
    <span class="chatmix-label chat tabular-nums">{$t('mixer.chat')} ({100 + Math.min(0, status.config.chatmix)}%)</span>
  </div>
</div>

<style>
  .mixer-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-bg-app);
    padding: 20px;
    gap: 16px;
    font-family: var(--font-sans);
    overflow-y: auto;
  }

  .device-header {
    display: flex;
    gap: 16px;
    background: var(--color-surface-1);
    padding: 14px 20px;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-depth-sm);
  }

  .device-col {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .label {
    font-size: 10px;
    font-weight: 800;
    color: var(--color-text-secondary);
    letter-spacing: 0.8px;
    text-transform: uppercase;
  }

  .master-select {
    background: var(--color-surface-3);
    color: var(--color-text-primary);
    border: 1px solid var(--color-border-default);
    padding: 8px 12px;
    font-size: 12px;
    font-weight: 500;
    font-family: var(--font-sans);
    border-radius: var(--radius-sm);
    outline: none;
    cursor: pointer;
    transition: border-color var(--transition-fast);
  }
  .master-select:hover {
    border-color: var(--color-border-bright);
  }

  .channel-strips {
    direction: ltr !important;
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 12px;
    flex: 1;
    min-height: 400px;
  }

  .strip {
    background: var(--color-surface-1);
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-md);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 16px 12px;
    transition: border-color var(--transition-fast), opacity var(--transition-fast), transform var(--transition-fast);
  }
  .strip:hover {
    border-color: var(--color-border-bright);
  }
  .strip.is-dimmed {
    opacity: 0.55;
  }
  .strip.is-eligible {
    border-style: dashed;
  }
  .strip.drag-over {
    border-style: solid;
    border-color: var(--color-accent-brand);
    background: var(--color-surface-hover);
    box-shadow: var(--shadow-depth-md);
    transform: translateY(-2px);
  }

  .empty-drop-hint {
    padding: 10px 4px;
    border: 1px dashed var(--color-border-default);
    border-radius: var(--radius-sm);
    font-size: 9px;
    font-weight: 700;
    color: var(--color-text-muted);
    text-align: center;
    letter-spacing: 0.5px;
    margin-top: 6px;
    transition: border-color var(--transition-fast), color var(--transition-fast);
  }
  .empty-drop-hint.highlight {
    border-color: var(--color-accent-brand);
    color: var(--color-accent-brand);
    background: var(--color-accent-brand-subtle);
  }

  .strip-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  .channel-name {
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.5px;
    color: var(--color-text-primary);
    text-transform: uppercase;
  }

  .preset-btn {
    width: 100%;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    color: var(--color-text-secondary);
    font-size: 10px;
    font-weight: 700;
    font-family: var(--font-sans);
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    margin-bottom: 16px;
    letter-spacing: 0.3px;
    transition: background-color var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }
  .preset-btn:hover {
    background: var(--color-surface-hover);
    color: var(--color-text-primary);
    border-color: var(--color-border-bright);
  }

  .fader-container {
    direction: ltr !important;
    position: relative;
    height: 180px;
    width: 44px;
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 6px;
    margin-bottom: 12px;
  }

  .fader-track {
    width: 8px;
    height: 100%;
    background: var(--color-surface-3);
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border-default);
    overflow: hidden;
    display: flex;
    align-items: flex-end;
  }

  /* Zero lag: thumb and fill track instantly without dragging latency */
  .fader-fill {
    width: 100%;
    transition: none;
  }

  .meter-track {
    width: 5px;
    height: 100%;
    background: var(--color-surface-3);
    border-radius: 2px;
    border: 1px solid var(--color-border-default);
    overflow: hidden;
    display: flex;
    align-items: flex-end;
  }

  .meter-fill {
    width: 100%;
    background: linear-gradient(to top, #10B981 0%, #10B981 70%, #F59E0B 85%, #EF4444 100%);
    background-size: 100% 180px;
    background-position: bottom;
    transition: height 0.05s linear;
    border-radius: 1px;
  }

  .vertical-slider {
    position: absolute;
    width: 180px;
    height: 36px;
    transform: rotate(-90deg);
    appearance: none;
    -webkit-appearance: none;
    background: transparent;
    outline: none;
    cursor: pointer;
    z-index: 2;
  }

  /* Sonar-authentic flat thumb: solid neutral-200 fill with 1px border and center accent indicator line */
  .vertical-slider::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 14px;
    height: 28px;
    background:
      linear-gradient(to right,
        var(--color-neutral-200) calc(50% - 1px),
        var(--color-accent-brand) calc(50% - 1px),
        var(--color-accent-brand) calc(50% + 1px),
        var(--color-neutral-200) calc(50% + 1px)
      );
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border-bright);
    cursor: grab;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .vertical-slider:hover::-webkit-slider-thumb {
    border-color: var(--color-accent-brand);
  }
  .vertical-slider:active::-webkit-slider-thumb {
    cursor: grabbing;
    border-color: var(--color-accent-brand);
  }

  .vertical-slider::-moz-range-thumb {
    width: 14px;
    height: 28px;
    background:
      linear-gradient(to right,
        var(--color-neutral-200) calc(50% - 1px),
        var(--color-accent-brand) calc(50% - 1px),
        var(--color-accent-brand) calc(50% + 1px),
        var(--color-neutral-200) calc(50% + 1px)
      );
    border-radius: var(--radius-sm);
    border: 1px solid var(--color-border-bright);
    cursor: grab;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .vertical-slider:hover::-moz-range-thumb {
    border-color: var(--color-accent-brand);
  }
  .vertical-slider:active::-moz-range-thumb {
    cursor: grabbing;
    border-color: var(--color-accent-brand);
  }

  .vol-text {
    font-size: 13px;
    font-weight: 700;
    color: var(--color-text-primary);
    font-family: var(--font-mono);
    font-variant-numeric: var(--tabular-nums);
    margin-bottom: 8px;
    letter-spacing: -0.5px;
  }

  .strip-btn-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin-bottom: 16px;
  }

  /* 44px hit targets on primary interactive buttons */
  .mute-btn {
    width: 44px;
    height: 44px;
    min-width: 44px;
    min-height: 44px;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    color: var(--color-text-secondary);
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }
  .mute-btn:hover {
    background: var(--color-surface-hover);
    color: var(--color-text-primary);
    border-color: var(--color-border-bright);
  }
  .mute-btn.muted {
    background: var(--color-accent-danger-subtle);
    border-color: var(--color-accent-danger);
    color: var(--color-accent-danger);
  }

  .strip-monitor-btn {
    width: 44px;
    height: 44px;
    min-width: 44px;
    min-height: 44px;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    color: var(--color-text-secondary);
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }
  .strip-monitor-btn:hover {
    background: var(--color-surface-hover);
    color: var(--color-text-primary);
    border-color: var(--color-border-bright);
  }
  .strip-monitor-btn.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand);
    color: var(--color-accent-brand);
  }

  .apps-box {
    width: 100%;
    flex: 1;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-subtle);
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    border-radius: var(--radius-sm);
    min-height: 90px;
  }

  .app-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--color-surface-1);
    border: 1px solid var(--color-border-default);
    padding: 6px 8px;
    border-radius: var(--radius-sm);
    font-size: 10px;
    font-weight: 600;
    color: var(--color-text-primary);
    cursor: grab;
    transition: border-color var(--transition-fast), background-color var(--transition-fast);
  }
  .app-chip:hover {
    border-color: var(--color-border-bright);
    background: var(--color-surface-hover);
  }
  .app-chip:active {
    cursor: grabbing;
  }
  .chip-color {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .chatmix-bar {
    direction: ltr;
    display: flex;
    align-items: center;
    gap: 16px;
    background: var(--color-surface-1);
    padding: 14px 24px;
    border: 1px solid var(--color-border-default);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-depth-sm);
  }

  .chatmix-label {
    font-size: 11px;
    font-weight: 800;
    font-family: var(--font-sans);
    letter-spacing: 0.5px;
  }
  .chatmix-label.game {
    color: var(--color-channel-game);
  }
  .chatmix-label.chat {
    color: var(--color-channel-chat);
  }

  .chatmix-slider {
    flex: 1;
    height: 6px;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-default);
    outline: none;
    appearance: none;
    -webkit-appearance: none;
    border-radius: var(--radius-sm);
  }
  .chatmix-slider::-webkit-slider-thumb {
    appearance: none;
    -webkit-appearance: none;
    width: 18px;
    height: 24px;
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    border-radius: var(--radius-sm);
    cursor: pointer;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .chatmix-slider:hover::-webkit-slider-thumb {
    border-color: var(--color-accent-brand);
  }

  .chatmix-slider::-moz-range-thumb {
    width: 18px;
    height: 24px;
    background: var(--color-neutral-200);
    border: 1px solid var(--color-border-bright);
    border-radius: var(--radius-sm);
    cursor: pointer;
    box-shadow: var(--shadow-depth-sm);
    transition: border-color var(--transition-fast);
  }
  .chatmix-slider:hover::-moz-range-thumb {
    border-color: var(--color-accent-brand);
  }
</style>
