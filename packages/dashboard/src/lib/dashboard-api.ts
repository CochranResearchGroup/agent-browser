"use client";

export const SERVICE_API_BASE = "/api/service";
export const CHAT_STATUS_API_URL = "/api/chat/status";
export const CHAT_API_URL = "/api/chat";
export const MODELS_API_URL = "/api/models";
export const APP_INTELLIGENCE_STATUS_API_URL = "/api/app-intelligence/status";
export const APP_INTELLIGENCE_INSPECT_API_URL = "/api/app-intelligence/inspect-workspace";
export const APP_INTELLIGENCE_OPERATOR_STATUS_API_URL = "/api/app-intelligence/operator/status";
export const APP_INTELLIGENCE_OPERATOR_TURN_API_URL = "/api/app-intelligence/operator/turn";
export const APP_INTELLIGENCE_OPERATOR_CONFIRM_API_URL = "/api/app-intelligence/operator/confirm";

export function sessionTabsApiUrl(port: number): string {
  return `/api/session-tabs?port=${encodeURIComponent(String(port))}`;
}
