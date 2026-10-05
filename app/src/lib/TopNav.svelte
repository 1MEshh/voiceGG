<script lang="ts">
  import type { ChannelId } from '../types';
  import { CHANNELS } from './theme';
  import { SlidersHorizontal, Gamepad2, MessageSquare, Music, Headphones, Mic, Settings, Radio, AlertTriangle } from '@lucide/svelte';

  export let activeTab: 'mixer' | ChannelId = 'mixer';
  export let activeGame: string | null = null;
  export let streamerMode: boolean = false;
  export let onSelectTab: (tab: 'mixer' | ChannelId) => void;
  export let onOpenSettings: () => void;
  export let onToggleStreamer: () => void;
  export let onPanicReset: () => void;

  const tabs: { id: 'mixer' | ChannelId; label: string; icon: any }[] = [
    { id: 'mixer', label: 'MIXER', icon: SlidersHorizontal },
    { id: 'game', label: 'GAME', icon: Gamepad2 },
    { id: 'chat', label: 'CHAT', icon: MessageSquare },
    { id: 'media', label: 'MEDIA', icon: Music },
    { id: 'aux', label: 'AUX', icon: Headphones },
    { id: 'mic', label: 'MIC', icon: Mic },
  ];
</script>

<header class="top-nav">
  <div class="brand">
    <div class="logo-badge">
      <SlidersHorizontal size={18} color="#22c55e" />
    </div>
    <span class="app-title">VoiceGG</span>
  </div>

  <nav class="nav-tabs">
    {#each tabs as tab}
      <button
        class="tab-btn"
        class:active={activeTab === tab.id}
        on:click={() => onSelectTab(tab.id)}
      >
        <svelte:component this={tab.icon} size={15} />
        <span>{tab.label}</span>
        {#if tab.id !== 'mixer'}
          <span
            class="channel-dot"
            style="background-color: {CHANNELS[tab.id]?.defaultColor || '#888'}"
          ></span>
        {/if}
      </button>
    {/each}
  </nav>

  <div class="actions">
    {#if activeGame}
      <div class="game-badge" title="Auto Game Detection: EQ preset applied">
        <span class="pulse-dot"></span>
        <span class="game-name">{activeGame}</span>
      </div>
    {/if}

    <button
      class="streamer-toggle"
      class:active={streamerMode}
      on:click={onToggleStreamer}
      title="Toggle Streamer Mode (OBS stream mix)"
    >
      <Radio size={14} />
      <span>STREAMER</span>
    </button>

    <button
      class="icon-btn panic-btn"
      on:click={onPanicReset}
      title="Panic Reset: Destroy virtual sinks and restore default PipeWire audio"
    >
      <AlertTriangle size={15} color="#ef4444" />
    </button>

    <button
      class="icon-btn settings-btn"
      on:click={onOpenSettings}
      title="Settings & Devices"
    >
      <Settings size={16} />
    </button>
  </div>
</header>

<style>
  .top-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 52px;
    padding: 0 16px;
    background-color: #13171d;
    border-bottom: 1px solid #232a35;
    user-select: none;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-badge {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: #1c2430;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid #2d3846;
  }

  .app-title {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: 0.5px;
    color: #f3f4f6;
  }

  .nav-tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    background: #181d24;
    padding: 3px;
    border-radius: 8px;
    border: 1px solid #232a35;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 14px;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: #94a3b8;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.5px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .tab-btn:hover {
    color: #f1f5f9;
    background: #202732;
  }

  .tab-btn.active {
    color: #ffffff;
    background: #2a3443;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  .channel-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .game-badge {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 4px 10px;
    background: #142a1e;
    border: 1px solid #22c55e44;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
    color: #4ade80;
  }

  .pulse-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: #22c55e;
    box-shadow: 0 0 6px #22c55e;
  }

  .game-name {
    max-width: 140px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .streamer-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: #1a2029;
    border: 1px solid #2a3443;
    border-radius: 6px;
    color: #94a3b8;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .streamer-toggle:hover {
    color: #f3f4f6;
    background: #232b37;
  }

  .streamer-toggle.active {
    color: #ec4899;
    border-color: #ec489988;
    background: #2a1525;
  }

  .icon-btn {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    background: #1a2029;
    border: 1px solid #2a3443;
    color: #94a3b8;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .icon-btn:hover {
    color: #f8fafc;
    background: #26303d;
  }

  .panic-btn:hover {
    background: #331515;
    border-color: #ef444466;
  }
</style>
