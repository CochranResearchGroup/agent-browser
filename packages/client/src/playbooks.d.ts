export type CallTool = (name: string, args: Record<string, unknown>) => Promise<{
  content: Array<{type: string; text?: string}>;
}>;
export interface VerifyLoginOptions {
  taskId: string;
  guideId: string;
  accountRef: string;
  organizationRef: string;
  maxObservationAgeMs?: number;
}
export interface RoutineResult {
  schemaVersion: 'agent-browser.routine-result.v1';
  guideId: string;
  routine: 'verify_login';
  status: 'completed' | 'waiting' | 'adaptation_required' | 'failed';
  reasonCode: string;
  effectState: 'no_effect';
  taskId: string;
  profileId: string;
  browserId: string;
  tabId: string;
  accountVerified: boolean;
  organizationVerified: boolean;
  observationElapsedMs: number;
}
export function createPlaybookClient(callTool: CallTool): {
  discover(operation?: 'siteguides' | 'inspect_guide' | 'verify_login'): Promise<unknown>;
  listGuides(taskId: string): Promise<{guides: string[]}>;
  inspectGuide(taskId: string, guideId: string): Promise<unknown>;
  verifyLogin(options: VerifyLoginOptions): Promise<RoutineResult>;
};
