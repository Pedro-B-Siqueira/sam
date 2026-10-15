<script lang="ts">
  import { onMount } from 'svelte';
  import { cleanItems, fetchCategories, fetchDiskUsage, fetchSettings, saveReport, saveSettings, scanCategory } from './lib/api/commands';
  import { onCleanProgress } from './lib/api/events';
  import { describeFailure, reportError, type FailureCause } from './lib/api/logger';
  import type { CategoryInfo, CleanOutcome, CleanProgress, DiskUsage, Settings, SettingsView } from './lib/api/types';
  import AppHeader from './lib/components/AppHeader.svelte';
  import CategoryRow from './lib/components/CategoryRow.svelte';
  import CleanResults from './lib/components/CleanResults.svelte';
  import ConfirmCleanModal from './lib/components/ConfirmCleanModal.svelte';
  import DiskOverview from './lib/components/DiskOverview.svelte';
  import GardenPlot from './lib/components/GardenPlot.svelte';
  import HelpDialog from './lib/components/HelpDialog.svelte';
  import ProjectsDialog from './lib/components/ProjectsDialog.svelte';
  import { formatBytes, pluralize } from './lib/format';
  import { plotColor } from './lib/labels';
  import { buildReport, reportFileName } from './lib/report';
  import { completedScans, type ScanState } from './lib/scan-state';
  import {
    initialSelection,
    selectedGroups,
    setItems,
    toCleanRequests,
    toggleItem,
    withCategorySelection,
    type Selection,
  } from './lib/selection';

  interface CleanReport {
    outcomes: CleanOutcome[];
    diskBefore: DiskUsage | null;
    diskAfter: DiskUsage | null;
    trashedBytes: number;
    finishedAt: Date;
  }

  let categories = $state.raw<CategoryInfo[]>([]);
  let scanStates = $state.raw<ReadonlyMap<string, ScanState>>(new Map());
  let selection = $state.raw<Selection>(new Map());
  let disk = $state.raw<DiskUsage | null>(null);
  let failure = $state<string | null>(null);
  let confirming = $state(false);
  let cleaning = $state(false);
  let progress = $state.raw<CleanProgress | null>(null);
  let cleaningStartedAt = $state(0);
  let report = $state.raw<CleanReport | null>(null);
  let projectsView = $state.raw<SettingsView | null>(null);
  let savingSettings = $state(false);
  let settingsError = $state<string | null>(null);
  let helpOpen = $state(false);
  let reportStatus = $state<string | null>(null);

  const scans = $derived(completedScans(scanStates));
  const groups = $derived(selectedGroups(categories, scans, selection));
  const foundBytes = $derived([...scans.values()].reduce((total, scan) => total + scan.totalBytes, 0));
  const selectedBytes = $derived(groups.reduce((total, group) => total + group.totalBytes, 0));
  const selectedCount = $derived(groups.reduce((total, group) => total + group.items.length, 0));
  const scanning = $derived(categories.length === 0 || [...scanStates.values()].some((state) => state.status === 'scanning'));

  function trashedBytes(outcomes: readonly CleanOutcome[]): number {
    const trashCategories = new Set(categories.filter((category) => category.explanation.reversibility === 'trash').map((category) => category.id));
    return outcomes
      .filter((outcome) => outcome.status === 'done' && trashCategories.has(outcome.categoryId))
      .reduce((total, outcome) => total + outcome.sizeBytes, 0);
  }

  function colorOf(categoryId: string): string {
    return plotColor(categories.findIndex((category) => category.id === categoryId));
  }

  function setScanState(categoryId: string, state: ScanState): void {
    scanStates = new Map(scanStates).set(categoryId, state);
  }

  async function refreshDisk(): Promise<void> {
    disk = await fetchDiskUsage().catch((cause: FailureCause) => {
      reportError('uso do disco', cause);
      return disk;
    });
  }

  async function scanOne(category: CategoryInfo): Promise<void> {
    setScanState(category.id, { status: 'scanning' });
    const scan = await scanCategory(category.id).catch((cause: FailureCause) => {
      reportError(`scan de ${category.id}`, cause);
      setScanState(category.id, { status: 'failed', message: describeFailure(cause) });
      return null;
    });
    if (!scan) return;
    setScanState(category.id, { status: 'done', scan });
    selection = withCategorySelection(selection, category.id, initialSelection(category, scan));
  }

  async function scanAll(): Promise<void> {
    failure = null;
    await Promise.all([refreshDisk(), ...categories.map(scanOne)]);
  }

  function setAll(category: CategoryInfo, selectAll: boolean): void {
    const scan = scans.get(category.id);
    const itemIds = selectAll && scan ? scan.items.map((item) => item.id) : [];
    selection = withCategorySelection(selection, category.id, itemIds);
  }

  async function confirmClean(): Promise<void> {
    cleaning = true;
    progress = null;
    cleaningStartedAt = Date.now();
    // Without updates the cleanup still runs; the view just stays at its first step.
    const stopListening = await onCleanProgress((update) => (progress = update)).catch((cause: FailureCause) => {
      reportError('andamento da limpeza', cause);
      return null;
    });
    const diskBefore = disk;
    const cleanedCategories = groups.map((group) => group.category);
    const outcomes = await cleanItems(toCleanRequests(groups)).catch((cause: FailureCause) => {
      reportError('limpeza', cause);
      failure = `A limpeza não começou: ${describeFailure(cause)}`;
      return null;
    });
    stopListening?.();
    await refreshDisk();
    cleaning = false;
    confirming = false;
    reportStatus = null;
    if (outcomes) report = { outcomes, diskBefore, diskAfter: disk, trashedBytes: trashedBytes(outcomes), finishedAt: new Date() };
    await Promise.all(cleanedCategories.map(scanOne));
  }

  async function saveReportFile(): Promise<void> {
    if (!report) return;
    const contents = buildReport({ ...report, categories });
    const savedPath = await saveReport(reportFileName(report.finishedAt), contents).catch((cause: FailureCause) => {
      reportError('salvar relatório', cause);
      reportStatus = `Não deu para salvar o relatório: ${describeFailure(cause)}`;
      return null;
    });
    if (savedPath) reportStatus = `Relatório salvo em ${savedPath}.`;
  }

  async function openProjects(): Promise<void> {
    settingsError = null;
    projectsView = await fetchSettings().catch((cause: FailureCause) => {
      reportError('configurações', cause);
      failure = `Não deu para abrir as pastas de projetos: ${describeFailure(cause)}`;
      return null;
    });
  }

  async function saveProjects(settings: Settings): Promise<void> {
    savingSettings = true;
    settingsError = null;
    const saved = await saveSettings(settings).catch((cause: FailureCause) => {
      reportError('salvar configurações', cause);
      settingsError = describeFailure(cause);
      return null;
    });
    savingSettings = false;
    if (!saved) return;
    projectsView = null;
    await scanAll();
  }

  onMount(() => {
    void fetchCategories()
      .then((loaded) => {
        categories = loaded;
        return scanAll();
      })
      .catch((cause: FailureCause) => {
        reportError('categorias', cause);
        failure = `O Sam não conseguiu abrir: ${describeFailure(cause)}`;
      });
  });
</script>

<div class="app">
  <AppHeader
    busy={scanning || cleaning}
    {scanning}
    onRescan={scanAll}
    onOpenProjects={openProjects}
    onOpenHelp={() => (helpOpen = true)}
  />

  <main>
    <div class="content">
      <DiskOverview usage={disk} {foundBytes} {selectedBytes} {scanning} />

      {#if failure}
        <p class="failure" role="alert">{failure}</p>
      {/if}

      <GardenPlot
        {categories}
        {scans}
        {selection}
        {scanning}
        disabled={cleaning}
        onSetItems={(categoryId, itemIds, selected) => (selection = setItems(selection, categoryId, itemIds, selected))}
      />

      <ul class="categories">
        {#each categories as category, index (category.id)}
          <CategoryRow
            {category}
            color={plotColor(index)}
            scanState={scanStates.get(category.id)}
            selected={selection.get(category.id)}
            disabled={cleaning}
            onToggleItem={(itemId) => (selection = toggleItem(selection, category.id, itemId))}
            onSetAll={(selectAll) => setAll(category, selectAll)}
          />
        {/each}
      </ul>
    </div>
  </main>

  <footer class="action-bar">
    <p class="selection-summary">
      {#if selectedCount > 0}
        <span class="figures">{formatBytes(selectedBytes)}</span> marcados em {pluralize(selectedCount, 'item', 'itens')}
      {:else}
        Nada marcado. Clique no canteiro ou marque itens na lista.
      {/if}
    </p>
    <button type="button" class="button-primary" disabled={selectedCount === 0 || cleaning || scanning} onclick={() => (confirming = true)}>
      Revisar e limpar
    </button>
  </footer>
</div>

{#if confirming}
  <ConfirmCleanModal
    {groups}
    {colorOf}
    {cleaning}
    {progress}
    startedAt={cleaningStartedAt}
    onCancel={() => (confirming = false)}
    onConfirm={confirmClean}
  />
{/if}

{#if projectsView}
  <ProjectsDialog
    view={projectsView}
    saving={savingSettings}
    error={settingsError}
    onCancel={() => (projectsView = null)}
    onSave={saveProjects}
  />
{/if}

{#if helpOpen}
  <HelpDialog onClose={() => (helpOpen = false)} />
{/if}

{#if report}
  <CleanResults
    outcomes={report.outcomes}
    diskBefore={report.diskBefore}
    diskAfter={report.diskAfter}
    trashedBytes={report.trashedBytes}
    {reportStatus}
    onSaveReport={saveReportFile}
    onClose={() => (report = null)}
  />
{/if}

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100%;
  }

  main {
    min-height: 0;
    overflow-y: auto;
  }

  .content {
    display: grid;
    gap: 28px;
    max-width: 980px;
    padding: 24px 32px 32px;
  }

  .failure {
    margin: 0;
    padding: 10px 14px;
    border-radius: var(--radius-control);
    background: color-mix(in srgb, var(--harm) 12%, transparent);
    color: var(--harm);
  }

  .categories {
    margin: 0;
    padding: 0;
    border-bottom: 1px solid var(--rule);
    list-style: none;
  }

  .action-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 12px 20px 12px 32px;
    border-top: 1px solid var(--rule);
    background: var(--paper);
  }

  .selection-summary {
    margin: 0;
    color: var(--ink-soft);
  }

  .selection-summary .figures {
    color: var(--ink);
    font-size: 16px;
    font-weight: 600;
  }
</style>
