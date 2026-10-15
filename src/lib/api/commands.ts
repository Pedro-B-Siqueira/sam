import { invoke } from '@tauri-apps/api/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { CategoryInfo, CategoryScan, CleanOutcome, CleanRequest, DiskUsage, Settings, SettingsView } from './types';

export function fetchCategories(): Promise<CategoryInfo[]> {
  return invoke<CategoryInfo[]>('categories_list');
}

export function fetchDiskUsage(): Promise<DiskUsage> {
  return invoke<DiskUsage>('disk_usage');
}

export function scanCategory(categoryId: string): Promise<CategoryScan> {
  return invoke<CategoryScan>('category_scan', { categoryId });
}

export function cleanItems(requests: CleanRequest[]): Promise<CleanOutcome[]> {
  return invoke<CleanOutcome[]>('items_clean', { requests });
}

export function fetchSettings(): Promise<SettingsView> {
  return invoke<SettingsView>('settings_get');
}

export function saveSettings(settings: Settings): Promise<SettingsView> {
  return invoke<SettingsView>('settings_update', { settings });
}

export function openFullDiskAccess(): Promise<void> {
  return invoke<void>('open_full_disk_access');
}

export function openTrash(): Promise<void> {
  return invoke<void>('open_trash');
}

/** Native folder picker; `null` when the user cancels. */
export async function pickFolder(title: string): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false, title });
  return typeof picked === 'string' ? picked : null;
}

/** Native save dialog, then writes the report there. `null` when the user cancels. */
export async function saveReport(fileName: string, contents: string): Promise<string | null> {
  const path = await save({ title: 'Salvar relatório da limpeza', defaultPath: fileName, filters: [{ name: 'Markdown', extensions: ['md'] }] });
  if (!path) return null;
  await invoke<void>('report_save', { path, contents });
  return path;
}
