<script lang="ts">
  import type { ChannelId } from '../types';
  import { SlidersHorizontal, Gamepad2, MessageSquare, Music, Headphones, Mic, Settings, Radio, AlertTriangle } from '@lucide/svelte';
  import { t } from './i18n';

  export let activeTab: 'mixer' | ChannelId = 'mixer';
  export let activeGame: string | null = null;
  export let streamerMode: boolean = false;
  export let onSelectTab: (tab: 'mixer' | ChannelId) => void;
  export let onOpenSettings: () => void;
  export let onToggleStreamer: () => void;
  export let onPanicReset: () => void;

  type NavTabId = 'mixer' | 'game' | 'chat' | 'media' | 'aux' | 'mic';

  const tabIds: NavTabId[] = ['mixer', 'game', 'chat', 'media', 'aux', 'mic'];
  const tabIcons: Record<NavTabId, any> = {
    mixer: SlidersHorizontal,
    game: Gamepad2,
    chat: MessageSquare,
    media: Music,
    aux: Headphones,
    mic: Mic,
  };
</script>

<header class="top-nav">
  <div class="brand">
    <img src="/logo.svg" alt="VoiceGG logo" class="logo-img" />
    <span class="app-title">VOICEGG</span>
  </div>

  <nav class="nav-tabs" aria-label="Audio Channels Navigation">
    {#each tabIds as tabId}
      {@const label = $t(`nav.${tabId}`)}
      <button
        class="tab-btn"
        class:active={activeTab === tabId}
        aria-current={activeTab === tabId ? 'page' : undefined}
        aria-label={$t('nav.navigate_to', { label })}
        on:click={() => onSelectTab(tabId)}
      >
        <svelte:component this={tabIcons[tabId]} size={15} />
        <span>{label}</span>
      </button>
    {/each}
  </nav>

  <div class="actions">
    {#if activeGame}
      <div class="game-badge" title={$t('nav.auto_game_detected', { game: activeGame })}>
        <span class="game-dot"></span>
        <span class="game-name">{activeGame}</span>
      </div>
    {/if}
    <button
      class="icon-btn"
      class:active={streamerMode}
      on:click={onToggleStreamer}
      aria-label={$t('nav.toggle_streamer')}
      title={$t('nav.streamer_mode')}
    >
      <Radio size={16} />
    </button>
    <button
      class="icon-btn panic-btn"
      on:click={onPanicReset}
      aria-label={$t('nav.panic_reset_aria')}
      title={$t('nav.panic_reset')}
    >
      <AlertTriangle size={16} />
    </button>
    <button
      class="icon-btn"
      on:click={onOpenSettings}
      aria-label={$t('nav.open_settings')}
      title={$t('nav.settings')}
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
    height: 48px;
    padding: 0 16px;
    background-color: var(--color-surface-1);
    border-bottom: 1px solid var(--color-border-default);
    user-select: none;
    font-family: var(--font-sans);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .logo-img {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-depth-sm);
    transition: transform var(--transition-fast);
  }
  .logo-img:hover {
    transform: scale(1.05);
  }
  .app-title {
    font-size: 13px;
    font-weight: 800;
    letter-spacing: 1px;
    color: var(--color-text-primary);
  }

  .nav-tabs {
    display: flex;
    height: 100%;
    gap: 2px;
  }
  .tab-btn {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 14px;
    height: 100%;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--color-text-muted);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: color var(--transition-fast), border-color var(--transition-fast), background-color var(--transition-fast);
    letter-spacing: 0.5px;
  }
  .tab-btn:hover {
    color: var(--color-text-secondary);
    background-color: rgba(255, 255, 255, 0.02);
  }
  .tab-btn.active {
    color: var(--color-text-primary);
    border-bottom: 2px solid var(--color-accent-brand);
    background-color: rgba(255, 82, 0, 0.04);
  }

  .actions {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .game-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--color-surface-3);
    border: 1px solid var(--color-border-bright);
    color: var(--color-accent-success);
    font-size: 10px;
    font-weight: 700;
    padding: 4px 8px;
    text-transform: uppercase;
    border-radius: var(--radius-sm);
    font-family: var(--font-sans);
  }
  .game-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--color-accent-success);
  }
  .game-name {
    letter-spacing: 0.5px;
  }

  .icon-btn {
    background: transparent;
    border: 1px solid transparent;
    color: var(--color-text-muted);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    border-radius: var(--radius-sm);
    transition: color var(--transition-fast), background-color var(--transition-fast), border-color var(--transition-fast);
  }
  .icon-btn:hover {
    color: var(--color-text-primary);
    background-color: var(--color-surface-3);
    border-color: var(--color-border-default);
  }
  .icon-btn.active {
    color: var(--color-accent-brand);
    background-color: var(--color-accent-brand-subtle);
    border-color: var(--color-accent-brand-border);
  }
  .panic-btn:hover {
    color: var(--color-accent-danger);
    background-color: var(--color-accent-danger-subtle);
    border-color: var(--color-accent-danger);
  }
</style>
