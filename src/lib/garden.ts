import type { CleanupItem } from './api/types';

/** One clickable area in the plot: a single item, or the small items of a category merged together. */
export interface GardenPlant {
  key: string;
  label: string;
  sizeBytes: number;
  itemIds: string[];
}

/** Items below this share of their category go into "Outros" instead of getting their own area. */
export const MIN_PLANT_SHARE = 0.08;
/** A plot column fits this many stacked areas with room for a name and a size in each. */
export const MAX_PLANTS = 4;

/** The biggest items of a category get their own area; the rest are merged into one "Outros". */
export function plantsFor(
  categoryId: string,
  items: readonly CleanupItem[],
  maxPlants: number = MAX_PLANTS,
  minShare: number = MIN_PLANT_SHARE,
): GardenPlant[] {
  const totalBytes = items.reduce((total, item) => total + item.sizeBytes, 0);
  const bySize = [...items].sort((left, right) => right.sizeBytes - left.sizeBytes);
  const own: CleanupItem[] = [];
  const merged: CleanupItem[] = [];
  for (const item of bySize) {
    const earnsOwnArea = own.length < maxPlants - 1 && item.sizeBytes >= totalBytes * minShare;
    (earnsOwnArea ? own : merged).push(item);
  }
  // A lone leftover keeps its own area; merging only pays off from two up.
  if (merged.length <= 1) return [...own, ...merged].map(asPlant);
  const others: GardenPlant = {
    key: `${categoryId}::outros`,
    label: `Outros (${merged.length} itens)`,
    sizeBytes: merged.reduce((total, item) => total + item.sizeBytes, 0),
    itemIds: merged.map((item) => item.id),
  };
  return [...own.map(asPlant), others];
}

function asPlant(item: CleanupItem): GardenPlant {
  return { key: item.id, label: item.label, sizeBytes: item.sizeBytes, itemIds: [item.id] };
}

/** What a plot cell has room to show, given its size in pixels. */
export interface CellLabel {
  showsName: boolean;
  showsSize: boolean;
  showsMark: boolean;
}

const NAME_MIN_WIDTH = 60;
const NAME_MIN_HEIGHT = 44;
const SIZE_MIN_WIDTH = 52;
const SIZE_MIN_HEIGHT = 26;
const MARK_MIN_SIDE = 44;

export function labelFor(width: number, height: number): CellLabel {
  return {
    showsName: width >= NAME_MIN_WIDTH && height >= NAME_MIN_HEIGHT,
    showsSize: width >= SIZE_MIN_WIDTH && height >= SIZE_MIN_HEIGHT,
    showsMark: width >= MARK_MIN_SIDE && height >= MARK_MIN_SIDE,
  };
}
