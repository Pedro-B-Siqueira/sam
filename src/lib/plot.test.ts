import { describe, expect, it } from 'vitest';
import { columnsPerRow, distribute, layoutRows, plantsPerColumn, rowsHeight, type ColumnSpacing, type PlacedPlant } from './plot';

const spacing: ColumnSpacing = { columnGap: 6, plantGap: 3, rowGap: 6, signHeight: 18, minColumnWidth: 76, minPlantHeight: 46, maxColumnsPerRow: 6 };

function sum(values: readonly number[]): number {
  return values.reduce((total, value) => total + value, 0);
}

function columnWidth(placed: readonly PlacedPlant[], bedKey: string): number {
  return placed.find((plant) => plant.bedKey === bedKey)?.width ?? 0;
}

describe('distribute', () => {
  it('splits in proportion when nobody falls below the minimum', () => {
    expect(distribute([3, 1], 400, 10)).toEqual([300, 100]);
  });

  it('raises small entries to the minimum and shares the rest by weight', () => {
    const sizes = distribute([16_000, 180, 170], 900, 76);

    expect(sizes[1]).toBe(76);
    expect(sizes[2]).toBe(76);
    expect(sizes[0]).toBeCloseTo(900 - 152);
  });

  it('gives equal shares when the minimums do not fit', () => {
    expect(distribute([5, 1, 1], 90, 40)).toEqual([30, 30, 30]);
  });

  it('always fills the whole length', () => {
    expect(sum(distribute([9, 5, 3, 1, 1, 0.5], 700, 76))).toBeCloseTo(700);
  });
});

describe('layoutRows in a single row', () => {
  const beds = [
    { key: 'npm', plants: [{ key: 'cacache', weight: 13_460 }, { key: 'npx', weight: 1_010 }] },
    { key: 'brew', plants: [{ key: 'cleanup', weight: 180 }] },
    { key: 'rust', plants: [{ key: 'claude-manager', weight: 16_490 }] },
    { key: 'empty', plants: [] },
  ];

  it('puts the biggest category first and skips empty ones', () => {
    const placed = layoutRows(beds, 980, 232, spacing);

    expect([...new Set(placed.map((plant) => plant.bedKey))]).toEqual(['rust', 'npm', 'brew']);
  });

  it('leaves room for the signs and stacks items without overlaps', () => {
    const placed = layoutRows(beds, 980, 232, spacing);

    expect(Math.min(...placed.map((plant) => plant.y))).toBe(spacing.signHeight);
    for (const plant of placed) expect(plant.y + plant.height).toBeLessThanOrEqual(232 + 1e-6);
    const npm = placed.filter((plant) => plant.bedKey === 'npm');
    expect(npm[1]?.y).toBeCloseTo((npm[0]?.y ?? 0) + (npm[0]?.height ?? 0) + spacing.plantGap);
  });
});

describe('layoutRows across several rows', () => {
  const elevenCategories = Array.from({ length: 11 }, (_, index) => ({
    key: `category-${index}`,
    plants: [
      { key: `big-${index}`, weight: 10_000 / (index + 1) ** 2 },
      { key: `small-${index}`, weight: 10 },
    ],
  }));

  it('wraps into more rows instead of squeezing columns', () => {
    const placed = layoutRows(elevenCategories, 696, 176, spacing);

    expect(columnsPerRow(696, spacing)).toBe(6);
    expect(columnsPerRow(400, spacing)).toBe(4);
    const rowOf = (plant: PlacedPlant): string => (plant.y >= 176 ? 'segunda' : 'primeira');
    expect(new Set(placed.map(rowOf))).toEqual(new Set(['primeira', 'segunda']));
    expect(rowsHeight(11, 696, 176, spacing)).toBe(176 * 2 + 6);
  });

  it('does not stretch the small categories of a later row', () => {
    const placed = layoutRows(elevenCategories, 696, 176, spacing);

    expect(columnWidth(placed, 'category-8')).toBeLessThanOrEqual(columnWidth(placed, 'category-7') + 1e-6);
    const secondRowWidth = sum(['category-8', 'category-9', 'category-10'].map((key) => columnWidth(placed, key)));
    expect(secondRowWidth).toBeLessThan(696 / 2);
  });

  it('keeps a big category clearly wider than a small one', () => {
    const placed = layoutRows(elevenCategories, 1016, 176, spacing);

    expect(columnWidth(placed, 'category-0')).toBeGreaterThan(columnWidth(placed, 'category-5') * 2);
  });

  it('never makes an area too small for a name, at any window width', () => {
    for (const width of [500, 696, 900, 1016, 1400]) {
      for (const plant of layoutRows(elevenCategories, width, 176, spacing)) {
        expect(plant.width).toBeGreaterThanOrEqual(spacing.minColumnWidth - 1e-9);
        expect(plant.height).toBeGreaterThanOrEqual(spacing.minPlantHeight - 1e-9);
        expect(plant.x + plant.width).toBeLessThanOrEqual(width + 1e-6);
      }
    }
  });

  it('knows how many areas fit under a sign', () => {
    expect(plantsPerColumn(232, spacing)).toBe(4);
    expect(plantsPerColumn(176, spacing)).toBe(3);
  });
});
