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
    defaultColor: '#E2E8F0', // Neutral Slate
    description: 'Final headphone audio mix with safety limiter',
  },
  game: {
    id: 'game',
    label: 'GAME',
    defaultColor: '#10B981', // Sonar Emerald
    description: 'Gaming audio, spatial audio, and game-specific EQ presets',
  },
  chat: {
    id: 'chat',
    label: 'CHAT',
    defaultColor: '#0EA5E9', // Sonar Sky
    description: 'Discord, voice calls, and vocal clarity enhancement',
  },
  media: {
    id: 'media',
    label: 'MEDIA',
    defaultColor: '#EC4899', // Sonar Magenta
    description: 'Music, browser videos, and media playback',
  },
  aux: {
    id: 'aux',
    label: 'AUX',
    defaultColor: '#8B5CF6', // Sonar Violet
    description: 'System sounds, notifications, and secondary audio',
  },
  mic: {
    id: 'mic',
    label: 'MIC',
    defaultColor: '#F97316', // Sonar Amber Orange
    description: 'Physical microphone processing with AI noise reduction',
  },
};

export const DEFAULT_CHANNEL_COLORS: Record<ChannelId, string> = {
  master: '#E2E8F0',
  game: '#10B981',
  chat: '#0EA5E9',
  media: '#EC4899',
  aux: '#8B5CF6',
  mic: '#F97316',
};
