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
    defaultColor: '#F8FAFC', // Crisp Silver/White
    description: 'Final headphone audio mix with safety limiter',
  },
  game: {
    id: 'game',
    label: 'GAME',
    defaultColor: '#00E599', // Sonar Tactical Mint/Green
    description: 'Gaming audio, spatial audio, and game-specific EQ presets',
  },
  chat: {
    id: 'chat',
    label: 'CHAT',
    defaultColor: '#00B4D8', // Sonar Crisp Cyan
    description: 'Discord, voice calls, and vocal clarity enhancement',
  },
  media: {
    id: 'media',
    label: 'MEDIA',
    defaultColor: '#3B82F6', // Sonar Clean Studio Blue
    description: 'Music, browser videos, and media playback',
  },
  aux: {
    id: 'aux',
    label: 'AUX',
    defaultColor: '#64748B', // Sonar Neutral Slate Steel
    description: 'System sounds, notifications, and secondary audio',
  },
  mic: {
    id: 'mic',
    label: 'MIC',
    defaultColor: '#FF5200', // SteelSeries Signature Orange
    description: 'Physical microphone processing with ClearCast AI noise cancellation',
  },
};

export const DEFAULT_CHANNEL_COLORS: Record<ChannelId, string> = {
  master: '#F8FAFC',
  game: '#00E599',
  chat: '#00B4D8',
  media: '#3B82F6',
  aux: '#64748B',
  mic: '#FF5200',
};

export const CHANNEL_COLOR_SWATCHES: string[] = [
  '#F8FAFC',
  '#00E599',
  '#00B4D8',
  '#3B82F6',
  '#64748B',
  '#FF5200',
];
