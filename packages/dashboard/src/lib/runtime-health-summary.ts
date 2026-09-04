export type RuntimeAccessFinding = {
  code: string;
  blocking: boolean;
  message: string;
};

export type RuntimeAccessHealth = {
  state: "allowed" | "attention" | "denied" | "unknown";
  findings: RuntimeAccessFinding[];
};

export function summarizeRuntimeAccess(access?: RuntimeAccessHealth): {
  ready: boolean;
  text: string | null;
} {
  if (!access) return { ready: true, text: null };
  const findingCount = access.findings.length;
  const blockingCount = access.findings.filter((finding) => finding.blocking).length;
  const ready = access.state === "allowed";
  if (findingCount === 0) {
    return { ready, text: `Access ${access.state}.` };
  }
  return {
    ready,
    text: `Access ${access.state}: ${findingCount} ${findingCount === 1 ? "finding" : "findings"}, ${blockingCount} blocking. Review Service diagnostics.`,
  };
}
