<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { ChannelId, SystemStatus, ActiveStream, ChannelMeter, Preset, EqConfig } from '../types';
  import { CHANNELS } from './theme';
  import {
    Volume2,
    VolumeX,
    SlidersHorizontal,
    Gamepad2,
    MessageSquare,
    Music,
    Headphones,
    Mic,
    Radio,
    GripVertical,
  } from '@lucide/svelte';
  import { t } from './i18n';
  import CustomSelect from './CustomSelect.svelte';
  import SonarFader from './SonarFader.svelte';
  import SonarSlider from './SonarSlider.svelte';
  import MiniEqPreview from './MiniEqPreview.svelte';

  export let status: SystemStatus;
  export let allPresets: Preset[] = [];
  export let streamerMode: boolean = false;
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
  export let onSetAppVolume: ((binary: string, volume: number) => void) | undefined = undefined;
  export let onSetPersonalMix: ((channel: ChannelId, enabled: boolean) => void) | undefined = undefined;
  export let onSetStreamMix: ((channel: ChannelId, enabled: boolean) => void) | undefined = undefined;

  let openAppPopoverStreamId: number | null = null;
  let popoverPos = { top: 0, left: 0 };
  let appMutedMap: Record<string, boolean> = {};
  let appPrevVolMap: Record<string, number> = {};

  function toggleAppPopover(streamId: number, event: MouseEvent) {
    if (openAppPopoverStreamId === streamId) {
      openAppPopoverStreamId = null;
      return;
    }
    const btn = event.currentTarget as HTMLElement;
    const rect = btn.getBoundingClientRect();
    let top = rect.bottom + 6;
    let left = Math.min(window.innerWidth - 180, Math.max(10, rect.left - 40));
    if (top + 100 > window.innerHeight) {
      top = Math.max(10, rect.top - 85);
    }
    popoverPos = { top, left };
    openAppPopoverStreamId = streamId;
  }

  function closeAppPopover() {
    openAppPopoverStreamId = null;
  }

  function getAppVolume(binary: string): number {
    if (status.config.app_volumes && status.config.app_volumes[binary] !== undefined) {
      return status.config.app_volumes[binary];
    }
    const rule = status.config.routing_rules?.find(
      (r) => r.binary_name.toLowerCase() === binary.toLowerCase()
    );
    return rule?.volume ?? 100;
  }

  function isAppMuted(binary: string): boolean {
    const key = binary.toLowerCase();
    if (appMutedMap[key] !== undefined) return appMutedMap[key];
    if (status.config.app_muted && status.config.app_muted[key] !== undefined) {
      return status.config.app_muted[key];
    }
    return getAppVolume(binary) === 0;
  }

  async function handleToggleAppMute(binary: string) {
    const key = binary.toLowerCase();
    const currentMuted = isAppMuted(binary);
    const newMuted = !currentMuted;
    appMutedMap[key] = newMuted;
    appMutedMap = { ...appMutedMap };

    if (newMuted) {
      const currentVol = getAppVolume(binary);
      if (currentVol > 0) appPrevVolMap[key] = currentVol;
    }

    try {
      await invoke('set_app_mute', { binaryName: binary, muted: newMuted });
    } catch {
      // Graceful fallback if backend set_app_mute is pending
      if (newMuted) {
        onSetAppVolume?.(binary, 0);
      } else {
        const restoreVol = appPrevVolMap[key] ?? 100;
        onSetAppVolume?.(binary, restoreVol);
      }
    }
  }

  $: playbackDeviceOptions = (() => {
    const list = (status.devices || []).filter((d) => d.device_type.toLowerCase() === 'sink');
    const options = [
      { value: 'default', label: $t('mixer.default_sink') },
      ...list.map((d) => ({ value: d.name, label: d.description || d.name })),
    ];
    const pref = status.config.preferred_output_device;
    if (pref && pref !== 'default' && !list.some((d) => d.name === pref)) {
      options.push({
        value: pref,
        label: `${pref} (${$t('mixer.device_disconnected', { default: 'Disconnected — Using System Default' })})`,
      });
    }
    return options;
  })();

  $: micDeviceOptions = (() => {
    const list = (status.devices || []).filter((d) => d.device_type.toLowerCase() === 'source');
    const options = [
      { value: 'default', label: $t('mixer.default_source') },
      ...list.map((d) => ({ value: d.name, label: d.description || d.name })),
    ];
    const pref = status.config.preferred_input_device;
    if (pref && pref !== 'default' && !list.some((d) => d.name === pref)) {
      options.push({
        value: pref,
        label: `${pref} (${$t('mixer.device_disconnected', { default: 'Disconnected — Using System Default' })})`,
      });
    }
    return options;
  })();

  function getChannelEq(ch: ChannelId): EqConfig | undefined {
    const presetId = status.config.active_presets[ch];
    if (!presetId) return undefined;
    if (status.config.custom_presets && status.config.custom_presets[presetId]) {
      return status.config.custom_presets[presetId].eq;
    }
    const found = allPresets.find((p) => p.id.toLowerCase() === presetId.toLowerCase());
    return found?.eq;
  }

  const channelList: ChannelId[] = ['master', 'game', 'chat', 'media', 'aux', 'mic'];
  const routableChannels: ChannelId[] = ['game', 'chat', 'media', 'aux'];
  const channelIcons: Record<ChannelId, any> = {
    master: SlidersHorizontal,
    game: Gamepad2,
    chat: MessageSquare,
    media: Music,
    aux: Headphones,
    mic: Mic,
  };

  /**
   * Ensures apps whose current_channel is 'Master' or null are displayed in their
   * classified channel (game, chat, media, aux) so no app is ever hidden.
   */
  function resolveStreamChannel(stream: ActiveStream): ChannelId {
    const raw = (stream.current_channel || '').toLowerCase();
    if (raw === 'game' || raw === 'chat' || raw === 'media' || raw === 'aux') {
      return raw as ChannelId;
    }

    const bin = (stream.binary_name || '').toLowerCase();
    const app = (stream.app_name || '').toLowerCase();
    const rule = (status.config.routing_rules || []).find(
      (r) => r.binary_name.toLowerCase() === bin || r.binary_name.toLowerCase() === app
    );
    if (rule) {
      const rc = (rule.target_channel || '').toLowerCase();
      if (rc === 'game' || rc === 'chat' || rc === 'media' || rc === 'aux') {
        return rc as ChannelId;
      }
    }

    const combined = `${bin} ${app}`;
    if (
      /discord|vesktop|webcord|teamspeak|ts3|mumble|slack|zoom|telegram|signal|element|skype|webrtc/i.test(
        combined
      )
    ) {
      return 'chat';
    }
    if (
      /spotify|vlc|mpv|firefox|chrom|brave|opera|vivaldi|zen|edge|cider|rhythmbox|amberol|elisa|audacious|totem|celluloid|kodi|plex/i.test(
        combined
      )
    ) {
      return 'media';
    }
    if (
      /steam|wine|proton|gamescope|lutris|heroic|prism|minecraft|java|cs2|dota|apex|overwatch|elden|cyberpunk|baldurs|hl2|unreal|unity|godot/i.test(
        combined
      )
    ) {
      return 'game';
    }
    return 'aux';
  }

  function getDisplayAppName(stream: ActiveStream): string {
    if (stream.app_name && stream.app_name !== 'unknown') {
      return stream.app_name;
    }
    if (stream.binary_name && stream.binary_name !== 'unknown') {
      return stream.binary_name;
    }
    return stream.app_name || stream.binary_name || 'Audio Stream';
  }

  function getRouteIdentifier(stream: ActiveStream): string {
    return stream.binary_name && stream.binary_name !== 'unknown'
      ? stream.binary_name
      : stream.app_name;
  }

  // Deterministic Pointer-Event Drag-and-Drop System for WebKitGTK
  interface DraggingApp {
    binary_name: string;
    app_name: string;
    fromChannel: ChannelId;
    x: number;
    y: number;
  }

  let pendingDrag: {
    binary_name: string;
    app_name: string;
    fromChannel: ChannelId;
    startX: number;
    startY: number;
  } | null = null;

  let draggingApp: DraggingApp | null = null;
  let dragOverChannel: ChannelId | null = null;

  function handleChipPointerDown(e: PointerEvent, stream: ActiveStream, fromChannel: ChannelId) {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest('button, input, select, .app-vol-popover, .chip-route-select')) {
      return;
    }
    pendingDrag = {
      binary_name: stream.binary_name || 'unknown',
      app_name: stream.app_name || stream.binary_name || 'App',
      fromChannel,
      startX: e.clientX,
      startY: e.clientY,
    };
  }

  function handleWindowPointerMove(e: PointerEvent) {
    if (!pendingDrag && !draggingApp) return;

    if (pendingDrag && !draggingApp) {
      const dist = Math.hypot(e.clientX - pendingDrag.startX, e.clientY - pendingDrag.startY);
      if (dist > 4) {
        draggingApp = {
          binary_name: pendingDrag.binary_name,
          app_name: pendingDrag.app_name,
          fromChannel: pendingDrag.fromChannel,
          x: e.clientX,
          y: e.clientY,
        };
        openAppPopover = null;
      }
    }

    if (draggingApp) {
      draggingApp.x = e.clientX;
      draggingApp.y = e.clientY;

      const el = document.elementFromPoint(e.clientX, e.clientY);
      const dropZone = el?.closest('[data-channel-drop]') as HTMLElement | null;
      const dropChannel = dropZone?.getAttribute('data-channel-drop') as ChannelId | null;

      if (
        dropChannel &&
        (dropChannel === 'game' ||
          dropChannel === 'chat' ||
          dropChannel === 'media' ||
          dropChannel === 'aux')
      ) {
        dragOverChannel = dropChannel;
      } else {
        dragOverChannel = null;
      }
    }
  }

  function handleWindowPointerUp() {
    if (draggingApp && dragOverChannel && dragOverChannel !== draggingApp.fromChannel) {
      const identifier =
        draggingApp.binary_name !== 'unknown' ? draggingApp.binary_name : draggingApp.app_name;
      onRouteApp(identifier, dragOverChannel);
    }
    pendingDrag = null;
    draggingApp = null;
    dragOverChannel = null;
  }
  $: activePopoverStream =
    openAppPopoverStreamId !== null
      ? (status.streams || []).find((s) => s.id === openAppPopoverStreamId)
      : null;
</script>

<svelte:window
  on:pointermove={handleWindowPointerMove}
  on:pointerup={handleWindowPointerUp}
  on:pointercancel={handleWindowPointerUp}
  on:click={closeAppPopover}
  on:keydown={(e) => e.key === 'Escape' && closeAppPopover()}
/>

<div class="mixer-view" class:is-dragging-app={draggingApp !== null}>
  <div class="device-header">
    <div class="device-col">
      <span class="label">{$t('mixer.playback_device')}</span>
      <CustomSelect
        value={status.config.preferred_output_device || 'default'}
        options={playbackDeviceOptions}
        onChange={(val) => onSetDevice('sink', val)}
      />
    </div>
    <div class="device-col">
      <span class="label">{$t('mixer.microphone')}</span>
      <CustomSelect
        value={status.config.preferred_input_device || 'default'}
        options={micDeviceOptions}
        onChange={(val) => onSetDevice('source', val)}
      />
    </div>
  </div>

  <div class="channel-strips">
    {#each channelList as ch}
      {@const info = CHANNELS[ch]}
      {@const chLabel = $t(`channels.${ch}`)}
      {@const vol = status.config.volumes[ch] ?? 100}
      {@const isMuted = status.config.muted[ch] ?? false}
      {@const presetId = status.config.active_presets[ch] ?? 'Default'}
      {@const isRoutable = ch === 'game' || ch === 'chat' || ch === 'media' || ch === 'aux'}
      {@const streams = isRoutable
        ? (status.streams || []).filter((s) => resolveStreamChannel(s) === ch)
        : []}

      <div
        class="strip"
        style="--ch-color: {info.defaultColor};"
        data-channel-drop={isRoutable ? ch : undefined}
        class:drag-over={dragOverChannel === ch}
        class:is-dimmed={draggingApp !== null && dragOverChannel !== ch && !isRoutable}
        class:is-eligible={draggingApp !== null && isRoutable}
        role="region"
        aria-label={$t('mixer.channel_strip_aria', { channel: chLabel })}
      >
        <div class="strip-top-bar" style="background-color: {info.defaultColor};"></div>

        <div class="strip-header">
          <svelte:component this={channelIcons[ch]} size={14} color={info.defaultColor} />
          <span class="channel-name">{chLabel}</span>
        </div>

        <button
          type="button"
          class="preset-btn"
          aria-label={$t('mixer.preset_aria', { preset: presetId })}
          on:click={() => onOpenPresetBrowser(ch)}
        >
          {presetId.replace(/^game_|^chat_|^media_|^mic_/, '').replace(/_/g, ' ').toUpperCase()}
        </button>

        <div class="mini-eq-wrap">
          <MiniEqPreview
            eqConfig={getChannelEq(ch)}
            accentColor={info.defaultColor}
            height={34}
          />
        </div>

        <div class="fader-container">
          <SonarFader
            value={vol}
            meter={meters?.[ch]}
            accentColor={info.defaultColor}
            disabled={isMuted}
            onChange={(newVol) => onSetVolume(ch, newVol)}
          />
        </div>

        <div class="strip-btn-row">
          <button
            type="button"
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
              type="button"
              class="strip-monitor-btn"
              class:active={micMonitorEnabled}
              on:click={() => onSetMicMonitor?.(!micMonitorEnabled, micMonitorVolume)}
              aria-label={micMonitorEnabled ? $t('mixer.disable_monitor') : $t('mixer.enable_monitor')}
              title={micMonitorEnabled ? $t('mixer.monitor_active') : $t('mixer.monitor_inactive')}
            >
              <Headphones size={15} />
            </button>
          {/if}
          {#if streamerMode}
            <div class="streamer-bus-row">
              <button
                type="button"
                class="streamer-bus-btn personal"
                class:active={status.config.personal_mix?.[ch] ?? true}
                on:click={() => onSetPersonalMix?.(ch, !(status.config.personal_mix?.[ch] ?? true))}
                title="Personal Monitor Mix"
                aria-label="Toggle Personal Mix for {chLabel}"
              >
                <Headphones size={13} />
              </button>
              <button
                type="button"
                class="streamer-bus-btn stream"
                class:active={status.config.stream_mix?.[ch] ?? true}
                on:click={() => onSetStreamMix?.(ch, !(status.config.stream_mix?.[ch] ?? true))}
                title="Stream Broadcast Mix"
                aria-label="Toggle Stream Broadcast Mix for {chLabel}"
              >
                <Radio size={13} />
              </button>
            </div>
          {/if}
        </div>

        {#if isRoutable}
          <div class="apps-box">
            <div class="apps-box-header">
              <span class="apps-box-title">ROUTED APPS</span>
              <span class="apps-count tabular-nums">{streams.length}</span>
            </div>
            {#each streams as stream (stream.id + '_' + stream.binary_name + '_' + stream.app_name)}
              {@const routeKey = getRouteIdentifier(stream)}
              {@const displayName = getDisplayAppName(stream)}
              {@const appVol = getAppVolume(routeKey)}
              {@const muted = isAppMuted(routeKey)}
              <!-- svelte-ignore a11y-no-static-element-interactions -->
              <div
                class="app-chip"
                class:is-being-dragged={draggingApp &&
                  (draggingApp.binary_name === stream.binary_name &&
                    draggingApp.app_name === stream.app_name)}
                aria-label={$t('mixer.route_app_aria', { name: displayName })}
                on:pointerdown={(e) => handleChipPointerDown(e, stream, ch)}
              >
                <span class="chip-drag-handle" title="Drag to another channel">
                  <GripVertical size={11} />
                </span>
                <span class="chip-color" style="background: {info.defaultColor};"></span>
                <span class="chip-name" title={displayName}>{displayName}</span>

                <select
                  class="chip-route-select"
                  aria-label="Route {displayName} to channel"
                  value={ch}
                  on:change={(e) => onRouteApp(routeKey, e.currentTarget.value as ChannelId)}
                >
                  {#each routableChannels as rCh}
                    <option value={rCh}>{rCh.toUpperCase()}</option>
                  {/each}
                </select>

                <!-- App Mute Toggle Button -->
                <button
                  type="button"
                  class="chip-mute-btn"
                  class:muted
                  title={muted ? `Unmute ${displayName}` : `Mute ${displayName}`}
                  aria-label={muted ? `Unmute ${displayName}` : `Mute ${displayName}`}
                  on:click|stopPropagation={() => handleToggleAppMute(routeKey)}
                >
                  {#if muted}
                    <VolumeX size={12} />
                  {:else}
                    <Volume2 size={12} />
                  {/if}
                </button>

                <button
                  type="button"
                  class="chip-vol-badge tabular-nums"
                  title={$t('mixer.app_volume_title', { name: displayName, vol: appVol })}
                  on:click|stopPropagation={(e) => toggleAppPopover(stream.id, e)}
                >
                  {appVol}%
                </button>
              </div>
            {/each}
            {#if streams.length === 0}
              <div class="empty-drop-hint" class:highlight={dragOverChannel === ch}>
                {draggingApp ? $t('mixer.drop_here') : $t('mixer.no_apps')}
              </div>
            {/if}
          </div>
        {:else if ch === 'master'}
          <div class="apps-box master-bus-info">
            <span class="bus-info-title">MASTER OUTPUT BUS</span>
            <span class="bus-info-desc">Sums Game, Chat, Media & Aux with -6 dBFS safety ceiling</span>
          </div>
        {:else if ch === 'mic'}
          <div class="apps-box master-bus-info">
            <span class="bus-info-title">CLEARCAST MIC BUS</span>
            <span class="bus-info-desc">Virtual Microphone Output (`VoiceGG_Mic_Out`)</span>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <!-- SteelSeries Sonar ChatMix Dock -->
  <div class="chatmix-dock-card">
    <div class="chatmix-header">
      <div class="chatmix-side game">
        <Gamepad2 size={16} color="var(--color-channel-game, #00E599)" />
        <span class="side-title">{$t('mixer.game')}</span>
        <span class="side-pct tabular-nums">{100 - Math.max(0, status.config.chatmix)}%</span>
      </div>
      <div class="chatmix-center-badge">
        <span class="badge-text">
          {status.config.chatmix === 0
            ? 'CHATMIX • BALANCED'
            : status.config.chatmix < 0
              ? 'CHATMIX • GAME FAVOR'
              : 'CHATMIX • CHAT FAVOR'}
        </span>
      </div>
      <div class="chatmix-side chat">
        <span class="side-pct tabular-nums">{100 + Math.min(0, status.config.chatmix)}%</span>
        <span class="side-title">{$t('mixer.chat')}</span>
        <MessageSquare size={16} color="var(--color-channel-chat, #00B4D8)" />
      </div>
    </div>

    <SonarSlider
      value={status.config.chatmix}
      min={-100}
      max={100}
      step={1}
      centerDetent={true}
      defaultValue={0}
      accentColor="var(--color-accent-brand, #FF5200)"
      onChange={(v) => onSetChatMix(v)}
    />
  </div>

  {#if draggingApp}
    <div
      class="drag-ghost-pill"
      style="position: fixed; left: {draggingApp.x + 14}px; top: {draggingApp.y - 16}px; pointer-events: none; z-index: 9999;"
    >
      <span
        class="ghost-dot"
        style="background: {CHANNELS[dragOverChannel || draggingApp.fromChannel]?.defaultColor || '#FF5200'};"
      ></span>
      <span class="ghost-name">
        {draggingApp.app_name !== 'unknown' ? draggingApp.app_name : draggingApp.binary_name}
      </span>
      <span
        class="ghost-target-badge"
        class:valid-drop={dragOverChannel !== null && dragOverChannel !== draggingApp.fromChannel}
      >
        {dragOverChannel ? `→ ${dragOverChannel.toUpperCase()}` : draggingApp.fromChannel.toUpperCase()}
      </span>
    </div>
  {/if}

  {#if activePopoverStream}
    {@const routeKey = getRouteIdentifier(activePopoverStream)}
    {@const displayName = getDisplayAppName(activePopoverStream)}
    {@const appVol = getAppVolume(routeKey)}
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="app-vol-popover fixed-popover"
      style="top: {popoverPos.top}px; left: {popoverPos.left}px;"
      role="presentation"
      on:click|stopPropagation
      on:pointerdown|stopPropagation
      on:keydown|stopPropagation
    >
      <div class="popover-header">
        <span class="popover-title">{displayName}</span>
        <span class="popover-val tabular-nums">{appVol}%</span>
      </div>
      <input
        type="range"
        min="0"
        max="150"
        value={appVol}
        on:input={(e) => onSetAppVolume?.(routeKey, Number(e.currentTarget.value))}
        class="popover-slider"
      />
    </div>
  {/if}
</div>

<style>
  .mixer-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--color-bg-app, #0C0E12);
    padding: 18px 20px;
    gap: 14px;
    font-family: var(--font-sans);
    overflow-y: auto;
  }

  .mixer-view.is-dragging-app {
    user-select: none;
    cursor: grabbing;
  }

  .device-header {
    display: flex;
    gap: 16px;
    background: var(--color-surface-1, #12151B);
    padding: 12px 18px;
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-md, 6px);
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
    color: var(--color-text-secondary, #94A3B8);
    letter-spacing: 0.8px;
    text-transform: uppercase;
  }

  .channel-strips {
    direction: ltr !important;
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 10px;
    flex: 1;
    min-height: 410px;
  }

  .strip {
    position: relative;
    background: var(--color-surface-1, #12151B);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-md, 6px);
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 14px 10px 10px;
    overflow: hidden;
    transition: border-color var(--transition-fast), background-color var(--transition-fast), opacity var(--transition-fast);
  }

  .strip-top-bar {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    opacity: 0.85;
  }

  .strip:hover {
    border-color: var(--color-border-bright, #2F3646);
  }

  .strip.is-dimmed {
    opacity: 0.5;
  }

  .strip.is-eligible {
    border-style: dashed;
    border-color: var(--color-border-bright, #2F3646);
  }

  .strip.drag-over {
    border-style: solid;
    border-color: var(--ch-color, #FF5200);
    background: var(--color-surface-2, #171B22);
  }

  .empty-drop-hint {
    padding: 12px 6px;
    border: 1px dashed var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    font-size: 9px;
    font-weight: 700;
    color: var(--color-text-muted, #64748B);
    text-align: center;
    letter-spacing: 0.5px;
    margin-top: 4px;
    transition: border-color var(--transition-fast), color var(--transition-fast);
  }

  .empty-drop-hint.highlight {
    border-color: var(--ch-color, #FF5200);
    color: var(--color-text-primary, #F8FAFC);
    background: rgba(255, 255, 255, 0.03);
  }

  .strip-header {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-bottom: 10px;
  }

  .channel-name {
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.6px;
    color: var(--color-text-primary, #F8FAFC);
    text-transform: uppercase;
  }

  .preset-btn {
    width: 100%;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 10px;
    font-weight: 700;
    font-family: var(--font-sans);
    padding: 6px 8px;
    border-radius: var(--radius-sm, 4px);
    cursor: pointer;
    margin-bottom: 10px;
    letter-spacing: 0.4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background-color var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .preset-btn:hover {
    background: var(--color-surface-hover, #262C38);
    color: var(--color-text-primary, #F8FAFC);
    border-color: var(--color-border-bright, #2F3646);
  }

  .mini-eq-wrap {
    width: 100%;
    margin-bottom: 10px;
  }

  .fader-container {
    direction: ltr !important;
    position: relative;
    width: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    margin-bottom: 8px;
  }

  .strip-btn-row {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin-bottom: 12px;
  }

  .mute-btn,
  .strip-monitor-btn {
    width: 38px;
    height: 36px;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    color: var(--color-text-secondary, #94A3B8);
    display: flex;
    justify-content: center;
    align-items: center;
    border-radius: var(--radius-sm, 4px);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .mute-btn:hover,
  .strip-monitor-btn:hover {
    background: var(--color-surface-hover, #262C38);
    color: var(--color-text-primary, #F8FAFC);
    border-color: var(--color-border-bright, #2F3646);
  }

  .mute-btn.muted {
    background: var(--color-accent-danger-subtle);
    border-color: var(--color-accent-danger, #EF4444);
    color: var(--color-accent-danger, #EF4444);
  }

  .strip-monitor-btn.active {
    background: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand, #FF5200);
    color: var(--color-accent-brand, #FF5200);
  }

  .apps-box {
    width: 100%;
    flex: 1;
    background: var(--color-neutral-950, #08090C);
    border: 1px solid var(--color-border-subtle, #1C2029);
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 5px;
    overflow-y: auto;
    border-radius: var(--radius-sm, 4px);
    min-height: 96px;
  }

  .apps-box-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 2px 4px;
    border-bottom: 1px solid var(--color-border-subtle, #1C2029);
    margin-bottom: 2px;
  }

  .apps-box-title {
    font-size: 8.5px;
    font-weight: 800;
    letter-spacing: 0.6px;
    color: var(--color-text-muted, #64748B);
  }

  .apps-count {
    font-size: 9px;
    font-weight: 700;
    color: var(--color-text-secondary, #94A3B8);
  }

  .master-bus-info {
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 10px 8px;
    gap: 4px;
  }

  .bus-info-title {
    font-size: 9px;
    font-weight: 800;
    letter-spacing: 0.6px;
    color: var(--color-text-secondary, #94A3B8);
  }

  .bus-info-desc {
    font-size: 9px;
    color: var(--color-text-muted, #64748B);
    line-height: 1.35;
  }

  .app-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    padding: 5px 6px;
    border-radius: var(--radius-sm, 4px);
    font-size: 10px;
    font-weight: 600;
    color: var(--color-text-primary, #F8FAFC);
    cursor: grab;
    position: relative;
    user-select: none;
    touch-action: none;
    transition: border-color var(--transition-fast), background-color var(--transition-fast), opacity var(--transition-fast);
  }

  .app-chip:hover {
    border-color: var(--color-border-bright, #2F3646);
    background: var(--color-surface-3, #1E232D);
  }

  .app-chip.is-being-dragged {
    opacity: 0.35;
    border-style: dashed;
  }

  .chip-drag-handle {
    display: flex;
    align-items: center;
    color: var(--color-text-muted, #64748B);
    flex-shrink: 0;
  }

  .chip-color {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }

  .chip-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 10px;
  }

  .chip-route-select {
    background: var(--color-surface-3, #1E232D);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-xs, 3px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 8.5px;
    font-weight: 800;
    font-family: var(--font-mono);
    padding: 1px 3px;
    cursor: pointer;
    outline: none;
  }

  .chip-route-select:hover {
    color: var(--color-text-primary, #F8FAFC);
    border-color: var(--color-border-bright, #2F3646);
  }

  .chip-mute-btn {
    background: transparent;
    border: none;
    color: var(--color-text-muted, #64748B);
    cursor: pointer;
    padding: 2px;
    border-radius: var(--radius-xs, 3px);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
  }

  .chip-mute-btn:hover {
    color: var(--color-text-primary, #F8FAFC);
    background: var(--color-surface-hover, #262C38);
  }

  .chip-mute-btn.muted {
    color: #EF4444;
  }

  .chip-vol-badge {
    background: var(--color-surface-3, #1E232D);
    border: 1px solid var(--color-border-subtle, #1C2029);
    border-radius: var(--radius-xs, 3px);
    padding: 1px 4px;
    font-size: 9px;
    font-weight: 700;
    color: var(--color-text-secondary, #94A3B8);
    cursor: pointer;
    transition: background-color var(--transition-fast), color var(--transition-fast), border-color var(--transition-fast);
  }

  .chip-vol-badge:hover {
    background: var(--color-surface-hover, #262C38);
    color: var(--color-text-primary, #F8FAFC);
    border-color: var(--color-border-bright, #2F3646);
  }

  .app-vol-popover.fixed-popover {
    position: fixed;
    width: 170px;
    background: var(--color-surface-1, #12151B);
    border: 1px solid var(--color-border-bright, #2F3646);
    border-radius: var(--radius-sm, 4px);
    padding: 10px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.7);
    z-index: 10000;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .popover-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 10px;
    font-weight: 700;
    color: var(--color-text-secondary, #94A3B8);
  }

  .popover-title {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .popover-val {
    color: var(--color-accent-brand, #FF5200);
    font-family: var(--font-mono);
  }

  .popover-slider {
    width: 100%;
    accent-color: var(--color-accent-brand, #FF5200);
    cursor: pointer;
  }

  .streamer-bus-row {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-inline-start: 4px;
  }

  .streamer-bus-btn {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm, 4px);
    border: 1px solid var(--color-border-bright, #2F3646);
    background: var(--color-surface-2, #171B22);
    color: var(--color-neutral-500, #64748B);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all var(--transition-fast, 120ms);
  }

  .streamer-bus-btn:hover {
    border-color: var(--color-neutral-400, #94A3B8);
    color: var(--color-text-primary, #F8FAFC);
  }

  .streamer-bus-btn.personal.active {
    background: rgba(0, 180, 216, 0.14);
    color: var(--color-channel-chat, #00B4D8);
    border-color: rgba(0, 180, 216, 0.4);
  }

  .streamer-bus-btn.stream.active {
    background: rgba(255, 82, 0, 0.14);
    color: var(--color-accent-brand, #FF5200);
    border-color: rgba(255, 82, 0, 0.4);
  }

  /* Floating Pointer-Event Drag Ghost Pill */
  .drag-ghost-pill {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--color-surface-1, #12151B);
    border: 1px solid var(--color-accent-brand, #FF5200);
    border-radius: var(--radius-sm, 4px);
    padding: 6px 10px;
    box-shadow: var(--shadow-depth-lg);
    font-family: var(--font-sans);
  }

  .ghost-dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }

  .ghost-name {
    font-size: 11px;
    font-weight: 700;
    color: var(--color-text-primary, #F8FAFC);
  }

  .ghost-target-badge {
    font-size: 9px;
    font-weight: 800;
    font-family: var(--font-mono);
    padding: 2px 5px;
    border-radius: var(--radius-xs, 3px);
    background: var(--color-surface-3, #1E232D);
    color: var(--color-text-secondary, #94A3B8);
    border: 1px solid var(--color-border-bright, #2F3646);
  }

  .ghost-target-badge.valid-drop {
    background: var(--color-accent-brand-subtle);
    color: var(--color-accent-brand, #FF5200);
    border-color: var(--color-accent-brand, #FF5200);
  }

  /* SteelSeries Sonar ChatMix Dock */
  .chatmix-dock-card {
    direction: ltr !important;
    display: flex;
    flex-direction: column;
    gap: 10px;
    background: var(--color-surface-1, #12151B);
    padding: 12px 22px;
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-md, 6px);
  }

  .chatmix-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .chatmix-side {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .side-title {
    font-size: 0.78rem;
    font-weight: 800;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .chatmix-side.game .side-title {
    color: var(--color-channel-game, #00E599);
  }

  .chatmix-side.chat .side-title {
    color: var(--color-channel-chat, #00B4D8);
  }

  .side-pct {
    font-size: 0.75rem;
    font-family: var(--font-mono);
    font-weight: 700;
    color: var(--color-neutral-400, #94A3B8);
  }

  .chatmix-center-badge {
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-bright, #2F3646);
    border-radius: var(--radius-sm, 4px);
    padding: 2px 10px;
  }

  .badge-text {
    font-size: 0.65rem;
    font-weight: 800;
    font-family: var(--font-mono);
    color: var(--color-text-secondary, #94A3B8);
    letter-spacing: 0.06em;
  }
</style>
