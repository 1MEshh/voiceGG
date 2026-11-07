<script lang="ts">
  import type { ChannelId } from '../types';
  import { CHANNELS } from './theme';
  import {
    SlidersHorizontal,
    Gamepad2,
    MessageSquare,
    Music,
    Headphones,
    Mic,
    Settings,
    Radio,
    Minimize2,
    Power,
  } from '@lucide/svelte';
  import { t } from './i18n';

  export let activeTab: 'mixer' | ChannelId = 'mixer';
  export let activeGame: string | null = null;
  export let streamerMode: boolean = false;
  export let onSelectTab: (tab: 'mixer' | ChannelId) => void;
  export let onOpenSettings: () => void;
  export let onToggleStreamer: () => void;
  export let onHideWindow: () => void = () => {};
  export let onQuitApp: () => void = () => {};

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

  const tabDisplayLabels: Record<NavTabId, string> = {
    mixer: 'MIXER',
    game: 'GAME',
    chat: 'CHAT',
    media: 'MEDIA',
    aux: 'AUX',
    mic: 'MIC',
  };

  function getTabAccent(tabId: NavTabId): string {
    if (tabId === 'mixer') return '#FF5200';
    return CHANNELS[tabId]?.defaultColor || '#FF5200';
  }
</script>

<header class="top-nav">
  <div class="brand">
    <div class="sonar-mark">
      <span class="brand-sonar">SONAR</span>
      <span class="brand-divider"></span>
      <span class="brand-voicegg">VOICEGG</span>
    </div>
    {#if activeGame}
      <div class="game-badge" title={$t('nav.auto_game_detected', { game: activeGame })}>
        <span class="game-dot"></span>
        <span class="game-name">{activeGame}</span>
      </div>
    {/if}
  </div>

  <nav class="nav-tabs" aria-label={$t('nav.sonar_nav')}>
    {#each tabIds as tabId}
      {@const label = $t(`nav.${tabId}`) || tabDisplayLabels[tabId]}
      {@const accent = getTabAccent(tabId)}
      <button
        type="button"
        class="tab-btn"
        class:active={activeTab === tabId}
        style="--tab-accent: {accent};"
        aria-current={activeTab === tabId ? 'page' : undefined}
        aria-label={$t('nav.navigate_to', { label })}
        on:click={() => onSelectTab(tabId)}
      >
        <svelte:component
          this={tabIcons[tabId]}
          size={14}
          color={activeTab === tabId ? accent : 'currentColor'}
        />
        <span>{label}</span>
      </button>
    {/each}
  </nav>

  <div class="actions">
    <button
      type="button"
      class="streamer-pill"
      class:active={streamerMode}
      on:click={onToggleStreamer}
      aria-label={$t('nav.toggle_streamer')}
      title={$t('nav.streamer_mode')}
    >
      <Radio size={13} />
      <span>{$t('nav.streamer_mode_btn')}</span>
      <span class="pill-led" class:on={streamerMode}></span>
    </button>

    <button
      type="button"
      class="icon-btn"
      on:click={onOpenSettings}
      aria-label={$t('nav.open_settings')}
      title={$t('nav.settings')}
    >
      <Settings size={15} />
    </button>

    <button
      type="button"
      class="icon-btn"
      on:click={onHideWindow}
      aria-label={$t('nav.minimize_to_bg')}
      title={$t('nav.minimize_to_bg')}
    >
      <Minimize2 size={15} />
    </button>

    <button
      type="button"
      class="quit-btn"
      on:click={onQuitApp}
      aria-label={$t('nav.quit_app')}
      title={$t('nav.quit_desc')}
    >
      <Power size={13} />
      <span>{$t('nav.quit_btn')}</span>
    </button>
  </div>
</header>

<style>
  .top-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
    padding: 0 18px;
    background-color: var(--color-neutral-950, #08090C);
    border-bottom: 1px solid var(--color-border-default, #232834);
    user-select: none;
    font-family: var(--font-sans);
    flex-shrink: 0;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .sonar-mark {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .brand-sonar {
    font-size: 14px;
    font-weight: 900;
    letter-spacing: 1.6px;
    color: var(--color-text-primary, #F8FAFC);
  }

  .brand-divider {
    width: 1px;
    height: 12px;
    background: var(--color-border-bright, #2F3646);
  }

  .brand-voicegg {
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 1.2px;
    color: var(--color-accent-brand, #FF5200);
  }

  .nav-tabs {
    display: flex;
    height: 100%;
    gap: 4px;
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
    color: var(--color-text-muted, #64748B);
    font-size: 11px;
    font-weight: 800;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: color var(--transition-fast), border-color var(--transition-fast), background-color var(--transition-fast);
    letter-spacing: 0.7px;
    text-transform: uppercase;
  }

  .tab-btn:hover {
    color: var(--color-text-primary, #F8FAFC);
    background-color: rgba(255, 255, 255, 0.02);
  }

  .tab-btn.active {
    color: var(--color-text-primary, #F8FAFC);
    border-bottom: 2px solid var(--tab-accent, #FF5200);
    background-color: var(--color-surface-1, #12151B);
  }

  .actions {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .game-badge {
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--color-surface-3, #1E232D);
    border: 1px solid var(--color-border-bright, #2F3646);
    color: var(--color-channel-game, #00E599);
    font-size: 10px;
    font-weight: 800;
    padding: 3px 8px;
    text-transform: uppercase;
    border-radius: var(--radius-sm, 4px);
    font-family: var(--font-sans);
  }

  .game-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background-color: var(--color-channel-game, #00E599);
  }

  .game-name {
    letter-spacing: 0.5px;
  }

  .streamer-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 10px;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.5px;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .streamer-pill:hover {
    color: var(--color-text-primary, #F8FAFC);
    border-color: var(--color-border-bright, #2F3646);
    background: var(--color-surface-3, #1E232D);
  }

  .streamer-pill.active {
    color: var(--color-text-primary, #F8FAFC);
    border-color: var(--color-accent-brand, #FF5200);
    background: var(--color-accent-brand-subtle);
  }

  .pill-led {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--color-neutral-500, #64748B);
  }

  .pill-led.on {
    background: var(--color-accent-brand, #FF5200);
  }

  .icon-btn {
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    color: var(--color-text-secondary, #94A3B8);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: var(--radius-sm, 4px);
    transition: color var(--transition-fast), background-color var(--transition-fast), border-color var(--transition-fast);
  }

  .icon-btn:hover {
    color: var(--color-text-primary, #F8FAFC);
    background-color: var(--color-surface-hover, #262C38);
    border-color: var(--color-border-bright, #2F3646);
  }

  .quit-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 10px;
    background: var(--color-surface-2, #171B22);
    border: 1px solid var(--color-border-default, #232834);
    border-radius: var(--radius-sm, 4px);
    color: var(--color-text-secondary, #94A3B8);
    font-size: 11px;
    font-weight: 700;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .quit-btn:hover {
    color: #F8FAFC;
    background: var(--color-accent-danger, #EF4444);
    border-color: var(--color-accent-danger, #EF4444);
  }
</style>
