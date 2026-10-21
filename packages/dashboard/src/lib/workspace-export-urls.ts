/** Remove URL credentials and variable payloads from copied or AI-bound evidence.
 * This export projection never mutates live URLs. Origin/path remain useful for
 * diagnosis; arbitrary secrets in prose or path segments are outside its scope. */
export function redactWorkspaceExportUrls<T>(value: T, field = ""): T {
  if (typeof value === "string") {
    // Opaque payloads can contain whitespace or markup, so omit the whole
    // export string rather than risking a partially copied payload.
    if (/\b(?:data|blob|javascript):/i.test(value)) return "[omitted-url]" as T;
    if (value.startsWith("/") && /url|uri|href|endpoint/i.test(field)) {
      try {
        const url = new URL(value, "https://export.invalid");
        return (value.startsWith("//") ? `//${url.host}${url.pathname}` : url.pathname) as T;
      } catch {
        return "[omitted-url]" as T;
      }
    }
    return value.replace(/(?:https?|wss?):\/\/[^\s"'<>]+/gi, (candidate) => {
      try {
        const url = new URL(candidate);
        if (!["http:", "https:", "ws:", "wss:"].includes(url.protocol)) return "[omitted-url]";
        url.username = "";
        url.password = "";
        url.search = "";
        url.hash = "";
        return url.toString();
      } catch {
        return "[omitted-url]";
      }
    }) as T;
  }
  if (Array.isArray(value)) return value.map((item) => redactWorkspaceExportUrls(item, field)) as T;
  if (typeof value === "object" && value !== null) {
    return Object.fromEntries(Object.entries(value).map(([key, nested]) => [key, redactWorkspaceExportUrls(nested, key)])) as T;
  }
  return value;
}
