export type IconName = 'refresh' | 'folder' | 'help' | 'plus';

/** 16×16 stroke paths, drawn with the current text color. */
export const ICON_PATHS: Readonly<Record<IconName, string>> = {
  refresh: 'M13.2 8.6A5.3 5.3 0 1 1 11.6 4.2M13.4 2.6v3.5H9.9',
  folder: 'M2 4.6A1.6 1.6 0 0 1 3.6 3h2.5l1.5 1.7h4.8A1.6 1.6 0 0 1 14 6.3v5.1a1.6 1.6 0 0 1-1.6 1.6H3.6A1.6 1.6 0 0 1 2 11.4z',
  help: 'M8 14.4A6.4 6.4 0 1 0 8 1.6a6.4 6.4 0 0 0 0 12.8zM6.3 6.3a1.8 1.8 0 1 1 2.5 1.7c-.5.2-.8.6-.8 1.1v.3M8 11.3v.2',
  plus: 'M8 3.2v9.6M3.2 8h9.6',
};
