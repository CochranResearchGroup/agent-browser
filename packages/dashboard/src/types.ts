export interface FrameMessage {
  type: "frame";
  data: string;
  metadata: {
    offsetTop: number;
    pageScaleFactor: number;
    deviceWidth: number;
    deviceHeight: number;
    scrollOffsetX: number;
    scrollOffsetY: number;
    timestamp: number;
  };
}

export interface StatusMessage {
  type: "status";
  connected: boolean;
  screencasting: boolean;
  viewportWidth: number;
  viewportHeight: number;
  engine?: string;
  recording?: boolean;
}

export interface CommandMessage {
  type: "command";
  action: string;
  id: string;
  params: Record<string, unknown>;
  timestamp: number;
}

export interface ResultMessage {
  type: "result";
  id: string;
  action: string;
  success: boolean;
  data: unknown;
  duration_ms: number;
  timestamp: number;
}

export interface ConsoleMessage {
  type: "console";
  level: string;
  text: string;
  timestamp: number;
  streamPort?: number | null;
  source?: "live-stream" | "retained-console" | "global-fallback";
  retainedKey?: string;
  args?: unknown[];
}

export interface UrlMessage {
  type: "url";
  url: string;
  timestamp: number;
}

export interface PageErrorMessage {
  type: "page_error";
  text: string;
  line: number | null;
  column: number | null;
  timestamp: number;
  streamPort?: number | null;
  source?: "live-stream" | "retained-console" | "global-fallback";
  retainedKey?: string;
}

export interface ErrorMessage {
  type: "error";
  message: string;
}

export interface TabInfo {
  index: number;
  title: string;
  url: string;
  type: string;
  active: boolean;
  targetId?: string | null;
}

export interface TabsMessage {
  type: "tabs";
  tabs: TabInfo[];
  timestamp: number;
}

export type StreamMessage =
  | FrameMessage
  | StatusMessage
  | CommandMessage
  | ResultMessage
  | ConsoleMessage
  | PageErrorMessage
  | ErrorMessage
  | UrlMessage
  | TabsMessage;

export type ActivityEvent = CommandMessage | ResultMessage | ConsoleMessage;
export type ConsoleEntry = ConsoleMessage | PageErrorMessage;

export interface ExtensionInfo {
  name: string;
  version: string;
  description?: string;
  path: string;
}

export interface SessionInfo {
  session: string;
  port: number;
  engine?: string;
  provider?: string;
  ownership?: string;
  addressability?: string;
  capabilities?: {
    inspect?: boolean;
    screenshot?: boolean;
    stream?: boolean;
    mutateRequiresBorrow?: boolean;
    lifecycle?: boolean;
    [key: string]: unknown;
  };
  borrow?: {
    state?: string | null;
    expiresAt?: string | null;
    owner?: string | null;
    [key: string]: unknown;
  };
  extensions?: ExtensionInfo[];
  detected?: boolean;
  cdpPort?: number;
  cdpUrl?: string;
  profilePath?: string;
  pid?: number;
  pending?: boolean;
  closing?: boolean;
}
