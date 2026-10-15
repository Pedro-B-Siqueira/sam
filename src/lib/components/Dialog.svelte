<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    width?: number;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let { title, width = 560, onClose, children, footer }: Props = $props();

  const titleId = `dialog-title-${Math.random().toString(36).slice(2, 8)}`;

  function handleKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={handleKey} />

<!-- svelte-ignore a11y_click_events_have_key_events (Escape is handled on the window) -->
<div class="backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onClose()}>
  <div class="dialog" role="dialog" aria-modal="true" aria-labelledby={titleId} style:width="min({width}px, 100%)">
    <header>
      <h2 id={titleId}>{title}</h2>
      <button type="button" class="close" aria-label="Fechar" onclick={onClose}>×</button>
    </header>
    <div class="content">{@render children()}</div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: grid;
    place-items: center;
    padding: 16px;
    background: var(--scrim);
    animation: backdrop-in var(--duration-quick) var(--ease) both;
  }

  .dialog {
    display: flex;
    flex-direction: column;
    max-height: min(700px, 100%);
    border-radius: var(--radius-panel);
    background: var(--ground);
    box-shadow:
      0 0 0 1px var(--rule),
      0 28px 60px color-mix(in srgb, var(--ink) 28%, transparent);
    overflow: hidden;
    animation: dialog-in var(--duration-quick) var(--ease) both;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 14px 10px 22px;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 19px;
    font-weight: 650;
  }

  .close {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--ink-soft);
    font-size: 20px;
    cursor: pointer;
  }

  .close:hover {
    background: var(--paper-sunk);
    color: var(--ink);
  }

  .content {
    padding: 4px 22px 18px;
    overflow-y: auto;
    user-select: text;
    -webkit-user-select: text;
  }

  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--rule);
    background: var(--paper);
  }

  @keyframes backdrop-in {
    from {
      opacity: 0;
    }
  }

  @keyframes dialog-in {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
