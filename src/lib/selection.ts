import type { CategoryInfo, CategoryScan, CleanRequest, CleanupItem } from './api/types';

/** Selected item ids per category id. Replaced on every change so Svelte sees a new value. */
export type Selection = ReadonlyMap<string, ReadonlySet<string>>;

export type CheckState = 'all' | 'some' | 'none';

export interface SelectedGroup {
  category: CategoryInfo;
  items: CleanupItem[];
  totalBytes: number;
}

/** Safe categories start fully selected; Review ones start empty so the user has to opt in. */
export function initialSelection(category: CategoryInfo, scan: CategoryScan): ReadonlySet<string> {
  return category.risk === 'safe' ? new Set(scan.items.map((item) => item.id)) : new Set();
}

export function withCategorySelection(selection: Selection, categoryId: string, itemIds: Iterable<string>): Selection {
  const next = new Map(selection);
  next.set(categoryId, new Set(itemIds));
  return next;
}

export function toggleItem(selection: Selection, categoryId: string, itemId: string): Selection {
  const current = new Set(selection.get(categoryId) ?? []);
  if (current.has(itemId)) {
    current.delete(itemId);
  } else {
    current.add(itemId);
  }
  return withCategorySelection(selection, categoryId, current);
}

/** Selects or clears several items of one category at once. */
export function setItems(selection: Selection, categoryId: string, itemIds: readonly string[], selected: boolean): Selection {
  const current = new Set(selection.get(categoryId) ?? []);
  for (const itemId of itemIds) {
    if (selected) {
      current.add(itemId);
    } else {
      current.delete(itemId);
    }
  }
  return withCategorySelection(selection, categoryId, current);
}

export function categoryCheckState(scan: CategoryScan, selected: ReadonlySet<string> | undefined): CheckState {
  const count = scan.items.filter((item) => selected?.has(item.id)).length;
  if (count === 0) return 'none';
  return count === scan.items.length ? 'all' : 'some';
}

export function sumBytes(items: readonly CleanupItem[]): number {
  return items.reduce((total, item) => total + item.sizeBytes, 0);
}

export function selectedGroups(
  categories: readonly CategoryInfo[],
  scans: ReadonlyMap<string, CategoryScan>,
  selection: Selection,
): SelectedGroup[] {
  return categories.flatMap((category) => {
    const scan = scans.get(category.id);
    const selectedIds = selection.get(category.id);
    if (!scan || !selectedIds) return [];
    const items = scan.items.filter((item) => selectedIds.has(item.id));
    return items.length === 0 ? [] : [{ category, items, totalBytes: sumBytes(items) }];
  });
}

export function toCleanRequests(groups: readonly SelectedGroup[]): CleanRequest[] {
  return groups.map((group) => ({ categoryId: group.category.id, itemIds: group.items.map((item) => item.id) }));
}
