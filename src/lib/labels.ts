import type { Reversibility, Risk, ScanIssue } from './api/types';

export const RISK_LABEL: Readonly<Record<Risk, string>> = {
  safe: 'Seguro',
  review: 'Revise antes',
};

export const REVERSIBILITY_LABEL: Readonly<Record<Reversibility, string>> = {
  regenerable: 'Recriado quando precisar',
  trash: 'Vai para a Lixeira',
  irreversible: 'Não tem volta',
};

const PLOT_COLOR_COUNT = 8;

/** Each category keeps one garden color, shared by its bed in the plot and its swatch in the list. */
export function plotColor(categoryIndex: number): string {
  return `var(--plot-${(categoryIndex % PLOT_COLOR_COUNT) + 1})`;
}

export function describeIssue(issue: ScanIssue): string {
  switch (issue.kind) {
    case 'toolMissing':
      return `${issue.tool} não está instalado neste Mac, então não há nada a limpar aqui.`;
    case 'notRunning':
      return `O ${issue.engine} está desligado. Abra o ${issue.engine} e clique em Medir de novo; o Sam não liga o Docker sozinho.`;
    case 'noProjectFolders':
      return 'Nenhuma pasta de projetos encontrada. Escolha onde ficam seus repositórios em Projetos, no topo da janela.';
    case 'noAccess':
      return `O macOS bloqueou ${issue.location}. Para liberar, abra Ajustes do Sistema, Privacidade e Segurança, Acesso Total ao Disco, e ative o Sam.`;
    case 'failed':
      return `Não deu para medir: ${issue.message}`;
  }
}
