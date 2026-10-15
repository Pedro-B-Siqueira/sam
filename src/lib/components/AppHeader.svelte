<script lang="ts">
  import Icon from './Icon.svelte';
  import Logo from './Logo.svelte';

  interface Props {
    busy: boolean;
    scanning: boolean;
    onRescan: () => void;
    onOpenProjects: () => void;
    onOpenHelp: () => void;
  }

  let { busy, scanning, onRescan, onOpenProjects, onOpenHelp }: Props = $props();
</script>

<!-- The top strip leaves the window buttons alone; the bar below holds the brand and the actions. -->
<header class="chrome" data-tauri-drag-region>
  <div class="bar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <Logo size={30} />
      <div class="names" data-tauri-drag-region>
        <span class="name">Sam</span>
        <span class="tagline">Limpeza de disco explicada</span>
      </div>
    </div>
    <nav class="actions" aria-label="Ações">
      <button type="button" class="button-tool" disabled={busy} onclick={onRescan}>
        <Icon name="refresh" />
        {scanning ? 'Medindo…' : 'Medir de novo'}
      </button>
      <button type="button" class="button-tool" disabled={busy} onclick={onOpenProjects}>
        <Icon name="folder" />
        Projetos
      </button>
      <button type="button" class="button-tool" onclick={onOpenHelp}>
        <Icon name="help" />
        Como funciona
      </button>
    </nav>
  </div>
</header>

<style>
  .chrome {
    padding-top: 28px;
    border-bottom: 1px solid var(--rule);
    background: var(--paper);
  }

  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 4px 20px 12px 32px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .names {
    display: grid;
    line-height: 1.15;
  }

  .name {
    font-family: var(--font-display);
    font-size: 19px;
    font-weight: 750;
  }

  .tagline {
    color: var(--ink-soft);
    font-size: 12px;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 6px;
  }
</style>
