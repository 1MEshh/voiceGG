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
    <img src="/logo.svg" alt="VoiceGG" class="logo-img" />
    <span class="app-title">VOICEGG</span>
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
      </button>
    {/each}
  </nav>

  <div class="actions">
    {#if activeGame}
      <div class="game-badge" title="Auto Game Detection">
        {activeGame}
      </div>
    {/if}
    <button class="icon-btn" class:active={streamerMode} on:click={onToggleStreamer} title="Streamer Mode">
      <Radio size={15} />
    </button>
    <button class="icon-btn panic-btn" on:click={onPanicReset} title="Panic Reset">
      <AlertTriangle size={15} />
    </button>
    <button class="icon-btn" on:click={onOpenSettings} title="Settings">
      <Settings size={15} />
    </button>
  </div>
</header>

<style>
  .top-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
    padding: 0 16px;
    background-color: #0d1117;
    border-bottom: 1px solid #1f2937;
    user-select: none;
    font-family: 'Inter', sans-serif;
  }
  .brand { display: flex; align-items: center; gap: 10px; }
  .logo-img { width: 26px; height: 26px; border-radius: 6px; box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4); transition: transform 0.15s ease; }
  .logo-img:hover { transform: scale(1.08); }
  .app-title { font-size: 14px; font-weight: 800; letter-spacing: 1px; color: #f8fafc; }
  
  .nav-tabs { display: flex; gap: 4px; }
  .tab-btn {
    display: flex; align-items: center; gap: 6px; padding: 6px 12px;
    background: transparent; border: none; color: #64748b; font-size: 11px;
    font-weight: 700; cursor: pointer; transition: all 0.1s; letter-spacing: 0.5px;
  }
  .tab-btn:hover { color: #cbd5e1; }
  .tab-btn.active { color: #f8fafc; border-bottom: 2px solid #38bdf8; }

  .actions { display: flex; gap: 8px; align-items: center; }
  .game-badge { background: #064e3b; color: #34d399; font-size: 10px; font-weight: 700; padding: 4px 8px; text-transform: uppercase; border-radius: 2px; }
  .icon-btn {
    background: transparent; border: none; color: #64748b; cursor: pointer;
    display: flex; align-items: center; justify-content: center; width: 28px; height: 28px;
  }
  .icon-btn:hover { color: #f8fafc; }
  .icon-btn.active { color: #c084fc; }
  .panic-btn:hover { color: #ef4444; }
</style>
