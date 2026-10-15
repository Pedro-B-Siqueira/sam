export interface AppErrorPayload {
  kind: string;
  message: string;
}

export interface DiskUsage {
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
}

export type Risk = 'safe' | 'review';

export type Reversibility = 'regenerable' | 'trash' | 'irreversible';

export interface Explanation {
  whatItIs: string;
  whyItGrows: string;
  effectOfCleaning: string;
  reversibility: Reversibility;
  reversibilityDetail: string;
  recreateCost: string;
  recommendation: string;
}

export interface CategoryInfo {
  id: string;
  title: string;
  /** Fits the sign above a narrow plot column. */
  shortTitle: string;
  summary: string;
  risk: Risk;
  explanation: Explanation;
}

export interface CleanupItem {
  id: string;
  label: string;
  location: string;
  sizeBytes: number;
  note: string | null;
}

export type ScanIssue =
  | { kind: 'toolMissing'; tool: string }
  | { kind: 'notRunning'; engine: string }
  | { kind: 'noAccess'; location: string }
  | { kind: 'noProjectFolders' }
  | { kind: 'failed'; message: string };

export interface CategoryScan {
  categoryId: string;
  items: CleanupItem[];
  totalBytes: number;
  footnote: string | null;
  issue: ScanIssue | null;
}

export interface CleanRequest {
  categoryId: string;
  itemIds: string[];
}

interface CleanOutcomeBase {
  categoryId: string;
  itemId: string;
  label: string;
  sizeBytes: number;
}

export type CleanOutcome =
  | (CleanOutcomeBase & { status: 'done'; output: string | null })
  | (CleanOutcomeBase & { status: 'failed'; message: string });

export interface Settings {
  /** Empty means "find them for me". */
  repoRoots: string[];
  idleDays: number;
}

export interface DetectedRoot {
  path: string;
  location: string;
  markers: string[];
}

export interface SettingsView {
  settings: Settings;
  homeDir: string;
  detectedRoots: DetectedRoot[];
  effectiveRoots: string[];
  gitAvailable: boolean;
}

/** Sent by the backend before each item is cleaned and once more when everything is done. */
export interface CleanProgress {
  completedItems: number;
  totalItems: number;
  completedBytes: number;
  totalBytes: number;
  currentLabel: string | null;
}
