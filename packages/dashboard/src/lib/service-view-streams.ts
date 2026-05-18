export type ServiceViewStream = {
  id?: string;
  provider?: string;
  url?: string | null;
  readOnly?: boolean;
};

const EMBEDDABLE_VIEW_STREAM_PROVIDERS = new Set([
  "external_url",
  "novnc",
  "rdp_gateway",
  "virtual_display_webrtc",
  "chrome_tab_webrtc",
]);

export function viewStreamLabel(stream: ServiceViewStream): string {
  return stream.provider?.replaceAll("_", " ") || "view stream";
}

export function canEmbedViewStream(stream: ServiceViewStream): boolean {
  return Boolean(stream.url && EMBEDDABLE_VIEW_STREAM_PROVIDERS.has(stream.provider ?? ""));
}

