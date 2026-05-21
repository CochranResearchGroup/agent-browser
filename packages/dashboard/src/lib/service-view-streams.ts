export type ServiceViewStream = {
  id?: string;
  provider?: string;
  controlInput?: string | null;
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

export function controlInputLabel(stream: ServiceViewStream): string {
  return stream.controlInput?.replaceAll("_", " ") || (stream.readOnly ? "view only" : "interactive");
}

export function canEmbedViewStream(stream: ServiceViewStream): boolean {
  return Boolean(stream.url && EMBEDDABLE_VIEW_STREAM_PROVIDERS.has(stream.provider ?? ""));
}

export function canControlViewStream(stream: ServiceViewStream): boolean {
  return stream.readOnly !== true && Boolean(stream.controlInput);
}

export function viewStreamCapabilityLabel(stream: ServiceViewStream): string {
  const view = viewStreamLabel(stream);
  const control = controlInputLabel(stream);
  return `${view} / ${control}`;
}
