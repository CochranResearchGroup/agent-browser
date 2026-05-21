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

export function canOpenViewStream(stream?: ServiceViewStream | null): boolean {
  return Boolean(stream && canEmbedViewStream(stream));
}

export function canOpenControlViewStream(stream?: ServiceViewStream | null): boolean {
  return Boolean(stream && canEmbedViewStream(stream) && canControlViewStream(stream));
}

export function viewStreamOpenTitle(stream?: ServiceViewStream | null): string {
  if (!stream) return "No service-owned view stream is registered for this browser.";
  if (!canEmbedViewStream(stream)) {
    if (!stream.url) return "This stream has no embeddable URL yet.";
    return "This stream provider is not embeddable in the dashboard.";
  }
  return `Open ${viewStreamLabel(stream)} in the dashboard.`;
}

export function viewStreamControlTitle(stream?: ServiceViewStream | null): string {
  if (!stream) return "No service-owned view stream is registered for this browser.";
  if (!canEmbedViewStream(stream)) return viewStreamOpenTitle(stream);
  if (!canControlViewStream(stream)) return "The service marked this stream as view-only or did not report a control input provider.";
  return `Focus the browser and open ${controlInputLabel(stream)} control.`;
}

export function viewStreamCapabilityLabel(stream: ServiceViewStream): string {
  const view = viewStreamLabel(stream);
  const control = controlInputLabel(stream);
  return `${view} / ${control}`;
}
