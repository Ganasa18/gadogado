export interface QaEventPayload {
  eventType: string;
  selector?: string;
  elementText?: string;
  value?: string;
  url?: string;
  metaJson?: string;
  runId?: string;
  origin?: string;
  recordingMode?: string;
}
