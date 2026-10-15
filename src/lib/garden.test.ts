import { describe, expect, it } from 'vitest';
import { labelFor, plantsFor } from './garden';
import { makeItem } from './test-fixtures';

describe('plantsFor', () => {
  it('keeps big items on their own and merges the small ones', () => {
    const plants = plantsFor('app-caches', [makeItem('chrome', 900), makeItem('a', 20), makeItem('b', 30), makeItem('c', 100)]);

    expect(plants.map((plant) => plant.label)).toEqual(['Item chrome', 'Item c', 'Outros (2 itens)']);
    expect(plants[2]).toMatchObject({ key: 'app-caches::outros', sizeBytes: 50, itemIds: ['b', 'a'] });
  });

  it('does not merge a single small item', () => {
    const plants = plantsFor('npm', [makeItem('cacache', 990), makeItem('npx', 10)]);
    expect(plants.map((plant) => plant.key)).toEqual(['cacache', 'npx']);
  });

  it('caps a category at four areas', () => {
    const plants = plantsFor('deps', ['a', 'b', 'c', 'd', 'e', 'f'].map((id) => makeItem(id, 100)));

    expect(plants.map((plant) => plant.label)).toEqual(['Item a', 'Item b', 'Item c', 'Outros (3 itens)']);
  });

  it('handles a category with one item', () => {
    expect(plantsFor('homebrew', [makeItem('cleanup', 177)])).toHaveLength(1);
  });
});

describe('labelFor', () => {
  it('shows name, size and marker when there is room', () => {
    expect(labelFor(76, 46)).toEqual({ showsName: true, showsSize: true, showsMark: true });
  });

  it('keeps at least the size in short cells', () => {
    expect(labelFor(56, 30)).toEqual({ showsName: false, showsSize: true, showsMark: false });
  });
});
