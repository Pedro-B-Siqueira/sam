export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WeightedEntry {
  key: string;
  weight: number;
}

export interface Bed {
  key: string;
  plants: WeightedEntry[];
}

export interface PlacedPlant extends Bounds {
  bedKey: string;
  plantKey: string;
}

export interface ColumnSpacing {
  columnGap: number;
  plantGap: number;
  rowGap: number;
  /** Room above each row for the signs that name the columns. */
  signHeight: number;
  /** Narrow enough to fit several categories, wide enough for a short name and a size. */
  minColumnWidth: number;
  /** Tall enough for a name over a size. */
  minPlantHeight: number;
  /** More columns than this leave no room for widths to differ, so sizes stop reading true. */
  maxColumnsPerRow: number;
}

interface WeightedBed extends Bed {
  weight: number;
}

/**
 * Splits `length` in proportion to `weights`, but never hands out less than `minimum`: entries
 * that would fall below it get exactly `minimum` and the rest share what is left. When even that
 * does not fit, everyone gets an equal share.
 */
export function distribute(weights: readonly number[], length: number, minimum: number): number[] {
  if (weights.length === 0) return [];
  if (minimum * weights.length >= length) return weights.map(() => length / weights.length);
  const pinned = new Set<number>();
  let sizes: number[] = [];
  let changed = true;
  while (changed) {
    const freeLength = length - pinned.size * minimum;
    const freeWeight = weights.reduce((total, weight, index) => (pinned.has(index) ? total : total + weight), 0);
    const freeCount = weights.length - pinned.size;
    sizes = weights.map((weight, index) => {
      if (pinned.has(index)) return minimum;
      return freeWeight > 0 ? (freeLength * weight) / freeWeight : freeLength / freeCount;
    });
    changed = false;
    sizes.forEach((size, index) => {
      if (!pinned.has(index) && size < minimum) {
        pinned.add(index);
        changed = true;
      }
    });
  }
  return sizes;
}

/** How many columns go side by side in `width`: as many as fit at `minColumnWidth`, up to `maxColumnsPerRow`. */
export function columnsPerRow(width: number, spacing: ColumnSpacing): number {
  const fitting = Math.floor((width + spacing.columnGap) / (spacing.minColumnWidth + spacing.columnGap));
  return Math.max(1, Math.min(fitting, spacing.maxColumnsPerRow));
}

/** How many areas of `minPlantHeight` fit stacked under the sign of a row of `rowHeight`. */
export function plantsPerColumn(rowHeight: number, spacing: ColumnSpacing): number {
  const usable = rowHeight - spacing.signHeight;
  return Math.max(1, Math.floor((usable + spacing.plantGap) / (spacing.minPlantHeight + spacing.plantGap)));
}

/** Total height `layoutRows` needs for `bedCount` non-empty beds. */
export function rowsHeight(bedCount: number, width: number, rowHeight: number, spacing: ColumnSpacing): number {
  const rows = Math.max(1, Math.ceil(bedCount / columnsPerRow(width, spacing)));
  return rows * rowHeight + (rows - 1) * spacing.rowGap;
}

/**
 * Beds (categories) become columns, biggest first, each as wide as its share; plants (items)
 * stack inside. When the columns do not all fit at `minColumnWidth`, they wrap into more rows.
 * Later rows hold the smallest beds and keep the first row's scale, so a tiny category never
 * stretches to look big. Callers must keep each bed to `plantsPerColumn(rowHeight)` plants.
 */
export function layoutRows(beds: readonly Bed[], width: number, rowHeight: number, spacing: ColumnSpacing): PlacedPlant[] {
  const planted = beds
    .map((bed) => ({ key: bed.key, plants: biggestFirst(bed.plants), weight: totalWeight(bed.plants) }))
    .filter((bed) => bed.weight > 0)
    .sort((left, right) => right.weight - left.weight);
  const perRow = columnsPerRow(width, spacing);
  const rows: WeightedBed[][] = [];
  for (let start = 0; start < planted.length; start += perRow) rows.push(planted.slice(start, start + perRow));

  const firstRow = rows[0] ?? [];
  const firstWidths = distribute(
    firstRow.map((bed) => bed.weight),
    width - gaps(firstRow.length, spacing.columnGap),
    spacing.minColumnWidth,
  );
  const pixelsPerWeight = (firstWidths[0] ?? 0) / (firstRow[0]?.weight ?? 1);

  return rows.flatMap((row, rowIndex) => {
    const widths = rowIndex === 0 ? firstWidths : scaledWidths(row, pixelsPerWeight, width, spacing);
    const top = rowIndex * (rowHeight + spacing.rowGap) + spacing.signHeight;
    return placeColumns(row, widths, { x: 0, y: top, width, height: rowHeight - spacing.signHeight }, spacing);
  });
}

function scaledWidths(row: readonly WeightedBed[], pixelsPerWeight: number, width: number, spacing: ColumnSpacing): number[] {
  const widths = row.map((bed) => Math.max(spacing.minColumnWidth, bed.weight * pixelsPerWeight));
  const usable = width - gaps(row.length, spacing.columnGap);
  if (widths.reduce((total, columnWidth) => total + columnWidth, 0) <= usable) return widths;
  return distribute(
    row.map((bed) => bed.weight),
    usable,
    spacing.minColumnWidth,
  );
}

function placeColumns(row: readonly WeightedBed[], widths: readonly number[], ground: Bounds, spacing: ColumnSpacing): PlacedPlant[] {
  let x = ground.x;
  return row.flatMap((bed, columnIndex) => {
    const width = widths[columnIndex] ?? 0;
    const columnX = x;
    x += width + spacing.columnGap;
    const heights = distribute(
      bed.plants.map((plant) => plant.weight),
      ground.height - gaps(bed.plants.length, spacing.plantGap),
      spacing.minPlantHeight,
    );
    let y = ground.y;
    return bed.plants.map((plant, plantIndex) => {
      const height = heights[plantIndex] ?? 0;
      const placed = { bedKey: bed.key, plantKey: plant.key, x: columnX, y, width, height };
      y += height + spacing.plantGap;
      return placed;
    });
  });
}

function biggestFirst(plants: readonly WeightedEntry[]): WeightedEntry[] {
  return plants.filter((plant) => plant.weight > 0).sort((left, right) => right.weight - left.weight);
}

function totalWeight(plants: readonly WeightedEntry[]): number {
  return plants.reduce((total, plant) => total + Math.max(plant.weight, 0), 0);
}

function gaps(count: number, gap: number): number {
  return gap * Math.max(count - 1, 0);
}
