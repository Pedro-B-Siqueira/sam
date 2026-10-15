<script lang="ts">
  import type { CategoryInfo } from '../api/types';
  import { formatBytes, pluralize } from '../format';
  import { describeIssue, REVERSIBILITY_LABEL, RISK_LABEL } from '../labels';
  import type { ScanState } from '../scan-state';
  import { categoryCheckState } from '../selection';
  import ExplainPanel from './ExplainPanel.svelte';

  interface Props {
    category: CategoryInfo;
    color: string;
    scanState: ScanState | undefined;
    selected: ReadonlySet<string> | undefined;
    disabled: boolean;
    onToggleItem: (itemId: string) => void;
    onSetAll: (selectAll: boolean) => void;
  }

  let { category, color, scanState, selected, disabled, onToggleItem, onSetAll }: Props = $props();

  let expanded = $state(false);
  let allCheckbox: HTMLInputElement | undefined = $state();

  const scan = $derived(scanState?.status === 'done' ? scanState.scan : null);
  const hasItems = $derived((scan?.items.length ?? 0) > 0);
  const checkState = $derived(scan ? categoryCheckState(scan, selected) : 'none');
  const detailsId = `category-details-${Math.random().toString(36).slice(2, 8)}`;

  $effect(() => {
    if (allCheckbox) allCheckbox.indeterminate = checkState === 'some';
  });
</script>

<li class="row" style:--bed-color={color}>
  <div class="line">
    <input
      bind:this={allCheckbox}
      type="checkbox"
      aria-label="Marcar tudo em {category.title}"
      checked={checkState === 'all'}
      disabled={disabled || !hasItems}
      onchange={() => onSetAll(checkState !== 'all')}
    />
    <span class="swatch" aria-hidden="true"></span>
    <div class="text">
      <h2>{category.title}</h2>
      <p class="summary">{category.summary}</p>
      <p class="traits">
        <span class="risk risk-{category.risk}">{RISK_LABEL[category.risk]}</span>
        <span>{REVERSIBILITY_LABEL[category.explanation.reversibility]}</span>
      </p>
    </div>
    <p class="size figures">
      {#if !scanState || scanState.status === 'scanning'}
        <span class="pending">medindo…</span>
      {:else if scan && hasItems}
        {formatBytes(scan.totalBytes)}
      {:else}
        <span class="pending">nada</span>
      {/if}
    </p>
  </div>

  {#if scan?.footnote}
    <p class="footnote">{scan.footnote}</p>
  {/if}

  {#if scanState?.status === 'failed'}
    <p class="notice">Não deu para medir: {scanState.message}</p>
  {:else if scan?.issue}
    <p class="notice">{describeIssue(scan.issue)}</p>
  {/if}

  <button
    type="button"
    class="button-disclosure disclosure"
    aria-expanded={expanded}
    aria-controls={detailsId}
    onclick={() => (expanded = !expanded)}
  >
    {expanded ? 'Fechar explicação' : 'Entender antes de limpar'}{hasItems && scan ? ` (${pluralize(scan.items.length, 'item', 'itens')})` : ''}
  </button>

  {#if expanded}
    <div class="details" id={detailsId}>
      <ExplainPanel explanation={category.explanation} />
      {#if scan && hasItems}
        <ul class="items">
          {#each scan.items as item (item.id)}
            <li>
              <label>
                <input type="checkbox" checked={selected?.has(item.id) ?? false} {disabled} onchange={() => onToggleItem(item.id)} />
                <span class="item-text">
                  <span class="item-label">{item.label}</span>
                  <span class="item-location">{item.location}</span>
                  {#if item.note}<span class="item-note">{item.note}</span>{/if}
                </span>
                <span class="item-size figures">{formatBytes(item.sizeBytes)}</span>
              </label>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</li>

<style>
  .row {
    display: grid;
    gap: 8px;
    padding: 18px 0;
    border-top: 1px solid var(--rule);
  }

  .line {
    display: grid;
    grid-template-columns: auto auto 1fr auto;
    align-items: start;
    gap: 12px;
  }

  .line > input {
    margin-top: 4px;
  }

  .swatch {
    width: 14px;
    height: 14px;
    margin-top: 4px;
    border-radius: 3px;
    background: var(--bed-color);
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 650;
    line-height: 1.3;
  }

  .summary {
    max-width: 60ch;
    margin: 2px 0 0;
    color: var(--ink-soft);
  }

  .traits {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin: 6px 0 0;
    color: var(--ink-soft);
    font-size: 12.5px;
  }

  .risk {
    font-weight: 700;
  }

  .risk-safe {
    color: var(--moss);
  }

  .risk-review {
    color: var(--caution);
  }

  .size {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    line-height: 1.2;
    white-space: nowrap;
  }

  .pending {
    color: var(--ink-soft);
    font-family: var(--font-text);
    font-size: 13px;
    font-weight: 400;
  }

  .footnote {
    margin: 0 0 0 53px;
    color: var(--ink-soft);
    font-size: 12.5px;
  }

  .notice {
    max-width: 70ch;
    margin: 0 0 0 53px;
    color: var(--caution);
  }

  .disclosure {
    justify-self: start;
    margin: 4px 0 0 53px;
  }

  .details {
    display: grid;
    gap: 16px;
    margin: 6px 0 0 53px;
    padding: 16px 18px;
    border-radius: var(--radius-panel);
    background: var(--paper);
  }

  .items {
    display: grid;
    margin: 0;
    padding: 12px 0 0;
    border-top: 1px solid var(--rule);
    list-style: none;
  }

  .items label {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: start;
    gap: 12px;
    margin: 0 -8px;
    padding: 8px;
    border-radius: var(--radius-control);
    cursor: pointer;
  }

  .items label:hover {
    background: var(--paper-sunk);
  }

  .items input {
    margin-top: 3px;
  }

  .item-text {
    display: grid;
    min-width: 0;
  }

  .item-label {
    font-weight: 650;
  }

  .item-location,
  .item-note {
    overflow: hidden;
    color: var(--ink-soft);
    font-size: 12.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-note {
    white-space: normal;
  }

  .item-size {
    white-space: nowrap;
  }
</style>
