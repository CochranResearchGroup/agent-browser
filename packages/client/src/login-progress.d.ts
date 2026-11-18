export interface LoginProgress {
  schemaVersion:1;
  outcome:'incomplete'|'verified';
  category:string;
  component:'auth'|'remote-view'|'application';
  waitingForUser:boolean;
  evidenceReferences:string[];
  nextAction:{operation:string;args:Record<string,string>}|null;
  loginId?:string;
  requestId?:string;
}
/** Read-only projection of an Auth public observation; never grants effects. */
export function summarizeLoginProgress(observation:Record<string,unknown>):LoginProgress;
