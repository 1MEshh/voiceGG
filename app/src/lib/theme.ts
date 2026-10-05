import type { ChannelId } from '../types';

export interface ChannelInfo {
  id: ChannelId;
  label: string;
  defaultColor: string;
  description: string;
}

export const CHANNELS: Record<ChannelId, ChannelInfo> = {
  master: {
    id: 'master',
    label: 'MASTER',
    defaultColor: '#a855f7', // Purple
    description: 'Final headphone audio mix with safety limiter',
  },
  game: {
    id: 'game',
    label: 'GAME',
    defaultColor: '#22c55e', // Emerald green
    description: 'Gaming audio, spatial audio, and game-specific EQ presets',
  },
  chat: {
    id: 'chat',
    label: 'CHAT',
    defaultColor: '#06b6d4', // Cyan
    description: 'Discord, voice calls, and vocal clarity enhancement',
  },
  media: {
    id: 'media',
    label: 'MEDIA',
    defaultColor: '#ec4899', // Pink
    description: 'Music, browser videos, and media playback',
  },
  aux: {
    id: 'aux',
    label: 'AUX',
    defaultColor: '#8b5cf6', // Violet
    description: 'System sounds, notifications, and secondary audio',
  },
  mic: {
    id: 'mic',
    label: 'MIC',
    defaultColor: '#f97316', // Orange
    description: 'Physical microphone processing with AI noise reduction',
  },
};

export const DEFAULT_CHANNEL_COLORS: Record<ChannelId, string> = {
  master: '#a855f7',
  game: '#22c55e',
  chat: '#06b6d4',
  media: '#ec4899',
  aux: '#8b5cf6',
  mic: '#f97316',
};
