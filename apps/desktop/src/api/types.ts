export type Protocol = 'chat' | 'responses';
export interface ModelOptions {
  endpoint?: string;
  protocol?: Protocol;
  fullUrl?: boolean;
  baseInstructions?: string;
  contextWindow?: number;
  contextNote?: string;
  reasoningLevels?: string[];
  defaultReasoningLevel?: string;
  reasoningNote?: string;
  inputModalities?: string[];
  parallelToolCalls?: boolean;
}
export interface ConnectionOptions {
  fullUrl?: boolean;
  endpointCandidates?: string[];
  modelOverrides?: Partial<Record<string, ModelOptions>>;
  chatReasoning?: {
    supportsThinking?: boolean;
    supportsEffort?: boolean;
    thinkingParam?: string;
    effortParam?: string;
    effortValueMode?: string;
    outputFormat?: string;
  } | null;
}
export interface RoutingSettings {
  remoteCompaction: boolean;
}
export interface Profile {
  id: string;
  name: string;
  presetId: string;
  endpoint: string;
  models: string[];
  protocol: Protocol;
  contextWindow: number;
  options?: ConnectionOptions;
}
export interface EndpointVariant {
  name: string;
  endpoint: string;
  model: string;
  protocol: Protocol;
  contextWindow: number;
  options?: ConnectionOptions;
}
export interface Preset {
  variants?: EndpointVariant[];
  id: string;
  name: string;
  endpoint: string;
  model: string;
  protocol: Protocol;
  keyUrl: string;
  docsUrl: string;
  envKey: string;
  contextWindow: number;
  options?: ConnectionOptions;
}
export interface Snapshot {
  profiles: Profile[];
  presets: Preset[];
  selectedProfiles: string[];
  enabled: boolean;
  routing: boolean;
  pendingReload: boolean;
  activeRequests: number;
  app: { installed: boolean; running: boolean };
  configPath: string;
  autostart: boolean;
  routingSettings?: RoutingSettings;
  trayFeedback?: { message: string; isError: boolean } | null;
}
export interface Receipt {
  ok: boolean;
  status: number | null;
  message: string;
  elapsedMs: number;
}
