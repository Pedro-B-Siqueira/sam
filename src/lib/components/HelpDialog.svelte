<script lang="ts">
  import { openFullDiskAccess } from '../api/commands';
  import { describeFailure, reportError, type FailureCause } from '../api/logger';
  import Dialog from './Dialog.svelte';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let openError = $state<string | null>(null);

  const RULES = [
    'Medir nunca apaga nada. Só o botão de limpar, depois da tela de confirmação, apaga.',
    'O Sam só apaga o que ele mesmo encontrou na última medição, dentro de pastas conhecidas, e nunca segue atalhos para fora delas.',
    'Dentro dos projetos, só entram pastas que o git ignora. Código e arquivos versionados nunca aparecem.',
    'Caches são apagados direto, porque mandar para a Lixeira não liberaria espaço. Cada item diz se é recriado sozinho ou se não tem volta.',
    'O que pede revisão vem desmarcado. O que é seguro vem marcado, e você pode desmarcar.',
  ];

  async function openAccessSettings(): Promise<void> {
    openError = null;
    await openFullDiskAccess().catch((cause: FailureCause) => {
      reportError('abrir Acesso Total ao Disco', cause);
      openError = `Não deu para abrir os Ajustes: ${describeFailure(cause)}`;
    });
  }
</script>

<Dialog title="Como o Sam funciona" width={580} {onClose}>
  <p class="intro">
    O Sam mede as pastas que ferramentas de desenvolvimento enchem sozinhas, como caches do npm e do Homebrew, node_modules e builds
    antigos, e explica cada uma antes de você decidir o que limpar.
  </p>

  <h3>Regras que ele segue</h3>
  <ul class="rules">
    {#each RULES as rule (rule)}
      <li>{rule}</li>
    {/each}
  </ul>

  <h3>Pastas bloqueadas pelo macOS</h3>
  <p>
    Algumas pastas só podem ser lidas por apps com Acesso Total ao Disco. Quando uma categoria avisar que o macOS bloqueou uma pasta,
    ative o Sam nessa lista e meça de novo.
  </p>
  <button type="button" class="button-quiet" onclick={openAccessSettings}>Abrir Acesso Total ao Disco</button>
  {#if openError}<p class="warning">{openError}</p>{/if}

  {#snippet footer()}
    <button type="button" class="button-primary" onclick={onClose}>Entendi</button>
  {/snippet}
</Dialog>

<style>
  .intro {
    max-width: 62ch;
    margin: 0 0 16px;
  }

  h3 {
    margin: 18px 0 6px;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 650;
  }

  p {
    max-width: 62ch;
    color: var(--ink-soft);
  }

  .rules {
    display: grid;
    gap: 6px;
    max-width: 62ch;
    margin: 0;
    padding-left: 18px;
    color: var(--ink-soft);
  }

  .rules li::marker {
    color: var(--moss);
  }

  .warning {
    color: var(--caution);
  }
</style>
