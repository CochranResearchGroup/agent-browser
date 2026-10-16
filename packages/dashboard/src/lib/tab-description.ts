/** Human description only. Keep the original URL separately for Properties/copy.
 * Opaque URL payloads are content, never observed page-title evidence. */
export function tabDescription(tab: { title?: string | null; taskLabel?: string | null; url?: string | null }): string {
  const title = tab.title?.trim();
  if (title) return title;
  const task = tab.taskLabel?.trim();
  if (task) return task;
  const url = tab.url?.trim();
  if (!url) return "Untitled tab";
  if (/^data:text\/html[;,]/i.test(url)) return "HTML document";
  if (/^data:/i.test(url)) return "Data document";
  if (/^blob:/i.test(url)) return "Browser document";
  if (/^javascript:/i.test(url)) return "Browser script";
  return url;
}
