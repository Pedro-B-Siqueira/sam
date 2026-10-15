import { describe, expect, it } from 'vitest';
import { formatBytes, formatElapsed, joinWithAnd, pluralize, shortenHome } from './format';

describe('formatBytes', () => {
  it('keeps small sizes in bytes', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(999)).toBe('999 B');
  });

  it('uses decimal units with one decimal place', () => {
    expect(formatBytes(1_500)).toBe('1,5 KB');
    expect(formatBytes(3_300_000_000)).toBe('3,3 GB');
  });

  it('drops the decimal from three digits up', () => {
    expect(formatBytes(228_000_000_000)).toBe('228 GB');
  });
});

describe('pluralize', () => {
  it('picks the form by count', () => {
    expect(pluralize(1, 'item', 'itens')).toBe('1 item');
    expect(pluralize(0, 'item', 'itens')).toBe('0 itens');
    expect(pluralize(3, 'item', 'itens')).toBe('3 itens');
  });
});

describe('shortenHome', () => {
  it('replaces the home folder with ~', () => {
    expect(shortenHome('/Users/me/field', '/Users/me')).toBe('~/field');
    expect(shortenHome('/Users/me', '/Users/me')).toBe('~');
    expect(shortenHome('/Users/meu/field', '/Users/me')).toBe('/Users/meu/field');
    expect(shortenHome('/opt/projects', '/Users/me')).toBe('/opt/projects');
  });
});

describe('joinWithAnd', () => {
  it('reads like a sentence', () => {
    expect(joinWithAnd([])).toBe('');
    expect(joinWithAnd(['mordor-2'])).toBe('mordor-2');
    expect(joinWithAnd(['eyeofsauron', 'mordor-2'])).toBe('eyeofsauron e mordor-2');
    expect(joinWithAnd(['a', 'b', 'c'])).toBe('a, b e c');
  });
});

describe('formatElapsed', () => {
  it('reads like a stopwatch', () => {
    expect(formatElapsed(0)).toBe('0:00');
    expect(formatElapsed(42_500)).toBe('0:42');
    expect(formatElapsed(185_000)).toBe('3:05');
  });
});
