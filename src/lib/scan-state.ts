import type { CategoryScan } from './api/types';

export type ScanState = { status: 'scanning' } | { status: 'done'; scan: CategoryScan } | { status: 'failed'; message: string };

export function completedScans(states: ReadonlyMap<string, ScanState>): ReadonlyMap<string, CategoryScan> {
  const scans = new Map<string, CategoryScan>();
  for (const [categoryId, state] of states) {
    if (state.status === 'done') scans.set(categoryId, state.scan);
  }
  return scans;
}
