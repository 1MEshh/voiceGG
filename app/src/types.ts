// TypeScript definitions mirroring VoiceGG Rust backend structures

export type ChannelId = 'master' | 'game' | 'chat' | 'media' | 'aux' | 'mic';

export interface AudioDevice {
  id: number;
  name: string;
  description: string;
  device_type: 'Sink' | 'Source';
  is_default: boolean;
}

export interface ActiveStream {
  id: number;
  app_name: string;
  binary_name: string;
  pid: number | null;
  current_channel: ChannelId | null;
}

export type FilterType =
  | 'peak'
  | 'low_shelf'
  | 'high_shelf'
  | 'low_pass'
  | 'high_pass'
  | 'notch';

export interface EqBand {
  freq_hz: number;
  gain_db: number;
  q: number;
  filter_type: FilterType;
  enabled: boolean;
}

export interface EqConfig {
  enabled: boolean;
  bands: EqBand[];
  bass_db: number;
  voice_db: number;
  treble_db: number;
}

export interface NoiseCancellerConfig {
  enabled: boolean;
  amount: number;
  vad_threshold: number;
}

export interface CompressorConfig {
  enabled: boolean;
  threshold_db: number;
  ratio: number;
  attack_ms: number;
  release_ms: number;
  makeup_gain_db: number;
}

export interface NoiseGateConfig {
  enabled: boolean;
  threshold_db: number;
  attack_ms: number;
  hold_ms: number;
  release_ms: number;
}

export interface LimiterConfig {
  enabled: boolean;
  ceiling_db: number;
  release_ms: number;
}

export interface Preset {
  schema: number;
  id: string;
  name: string;
  description: string;
  category: 'game' | 'chat' | 'media' | 'mic' | 'custom';
  tags: string[];
  eq: EqConfig;
  noise_canceller?: NoiseCancellerConfig;
  compressor?: CompressorConfig;
  noise_gate?: NoiseGateConfig;
  limiter?: LimiterConfig;
}

export interface AppRouteRule {
  binary_name: string;
  target_channel: ChannelId;
  volume: number;
}

export interface VoiceggConfig {
  active_presets: Record<ChannelId, string>;
  volumes: Record<ChannelId, number>;
  muted: Record<ChannelId, boolean>;
  chatmix: number;
  routing_rules: AppRouteRule[];
  auto_game_detection: boolean;
}

export interface SystemStatus {
  config: VoiceggConfig;
  devices: AudioDevice[];
  streams: ActiveStream[];
  active_game: string | null;
}
