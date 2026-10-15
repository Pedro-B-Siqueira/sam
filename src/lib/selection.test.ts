import { describe, expect, it } from 'vitest';
import {
  categoryCheckState,
  initialSelection,
  setItems,
  selectedGroups,
  toCleanRequests,
  toggleItem,
  withCategorySelection,
  type Selection,
} from './selection';
import { makeCategory, makeItem, makeScan } from './test-fixtures';

const safe = makeCategory('npm-cache', 'safe');
const review = makeCategory('node-modules', 'review');
const safeScan = makeScan('npm-cache', [makeItem('a', 100), makeItem('b', 50)]);
const reviewScan = makeScan('node-modules', [makeItem('c', 400)]);
const scans = new Map([
  ['npm-cache', safeScan],
  ['node-modules', reviewScan],
]);

describe('initialSelection', () => {
  it('selects every item of a safe category', () => {
    expect([...initialSelection(safe, safeScan)]).toEqual(['a', 'b']);
  });

  it('selects nothing in a category that needs review', () => {
    expect(initialSelection(review, reviewScan).size).toBe(0);
  });
});

describe('toggleItem', () => {
  it('adds and removes without touching the previous selection', () => {
    const empty: Selection = new Map();
    const selected = toggleItem(empty, 'npm-cache', 'a');
    const unselected = toggleItem(selected, 'npm-cache', 'a');

    expect(empty.size).toBe(0);
    expect(selected.get('npm-cache')?.has('a')).toBe(true);
    expect(unselected.get('npm-cache')?.has('a')).toBe(false);
  });
});

describe('setItems', () => {
  it('selects and clears a group of items together', () => {
    const selected = setItems(new Map(), 'npm-cache', ['a', 'b'], true);
    const cleared = setItems(selected, 'npm-cache', ['a'], false);

    expect([...(selected.get('npm-cache') ?? [])]).toEqual(['a', 'b']);
    expect([...(cleared.get('npm-cache') ?? [])]).toEqual(['b']);
  });
});

describe('categoryCheckState', () => {
  it('tells all, some and none apart', () => {
    expect(categoryCheckState(safeScan, new Set(['a', 'b']))).toBe('all');
    expect(categoryCheckState(safeScan, new Set(['a']))).toBe('some');
    expect(categoryCheckState(safeScan, undefined)).toBe('none');
  });
});

describe('selectedGroups and toCleanRequests', () => {
  it('groups only selected items, in category order, with their total', () => {
    let selection: Selection = withCategorySelection(new Map(), 'node-modules', ['c']);
    selection = withCategorySelection(selection, 'npm-cache', ['b']);

    const groups = selectedGroups([safe, review], scans, selection);

    expect(groups.map((group) => [group.category.id, group.totalBytes])).toEqual([
      ['npm-cache', 50],
      ['node-modules', 400],
    ]);
    expect(toCleanRequests(groups)).toEqual([
      { categoryId: 'npm-cache', itemIds: ['b'] },
      { categoryId: 'node-modules', itemIds: ['c'] },
    ]);
  });

  it('ignores ids that are no longer in the scan', () => {
    const selection = withCategorySelection(new Map(), 'npm-cache', ['gone']);
    expect(selectedGroups([safe], scans, selection)).toEqual([]);
  });
});
