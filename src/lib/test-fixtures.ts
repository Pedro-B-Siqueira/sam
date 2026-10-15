import type { CategoryInfo, CategoryScan, CleanupItem, Reversibility, Risk } from './api/types';

export function makeCategory(id: string, risk: Risk = 'safe', reversibility: Reversibility = 'regenerable'): CategoryInfo {
  return {
    id,
    title: `Categoria ${id}`,
    shortTitle: id,
    summary: `Resumo de ${id}`,
    risk,
    explanation: {
      whatItIs: `O que é ${id}`,
      whyItGrows: `Por que ${id} cresce`,
      effectOfCleaning: `Efeito de limpar ${id}`,
      reversibility,
      reversibilityDetail: `Detalhe de reversão de ${id}`,
      recreateCost: `Custo de recriar ${id}`,
      recommendation: `Recomendação para ${id}`,
    },
  };
}

export function makeItem(id: string, sizeBytes: number): CleanupItem {
  return { id, label: `Item ${id}`, location: `~/${id}`, sizeBytes, note: null };
}

export function makeScan(categoryId: string, items: CleanupItem[]): CategoryScan {
  return { categoryId, items, totalBytes: items.reduce((total, item) => total + item.sizeBytes, 0), footnote: null, issue: null };
}
