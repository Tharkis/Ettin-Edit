// Types for the Ettin Edit plugin API (ettin.js), for editors that check
// types or complete names. Ettin Edit copies this file next to new plugins;
// in main.js, `/** @type {import('./ettin')} */` before `require('ettin')`
// picks it up.

export type MarkStyle = 'underline-squiggle' | 'background' | 'gutter-icon';
export type MarkColor = 'error' | 'warning' | 'info' | 'accent';
export type EventName = 'opened' | 'saved' | 'closed' | 'activated' | 'changed' | 'caret'
  | 'shutdown' | 'settings.changed';
export type Item = Record<string, unknown>;
export type Options = Record<string, unknown>;

export const info: Record<string, unknown>;
/** The plugin's settings, filled in before any command, event or transform runs. */
export const settings: Record<string, unknown>;

export class EttinError extends Error {}
export class TransformError extends Error {
  constructor(message: string, where?: { line?: number; column?: number });
  line?: number;
  column?: number;
}

export interface Selection { start: number; end: number; anchor: number; head: number; text: string; primary: boolean }

export class Document {
  readonly id: number;
  readonly title: string;
  readonly path: string | null;
  readonly language: string;
  readonly modified: boolean;
  readonly readOnly: boolean;
  readonly length: number;
  readonly lineCount: number;
  readonly encoding: string;
  readonly lineEnding: string;
  readonly caret: { line: number; column: number; offset: number; anchor: number };
  refresh(): Promise<Document>;
  getText(start?: number, end?: number): Promise<string>;
  setText(text: string): Promise<void>;
  replace(start: number, end: number, text: string): Promise<void>;
  insert(text: string): Promise<void>;
  getSelections(): Promise<Selection[]>;
  getSelectedText(): Promise<string>;
  replaceSelections(text: string | string[]): Promise<void>;
  select(start: number, end?: number): Promise<void>;
  selectRanges(ranges: Array<[number, number]>): Promise<void>;
  goTo(line: number, column?: number): Promise<void>;
  save(): Promise<boolean>;
  activate(): Promise<void>;
  close(): Promise<void>;
  mark(id: string, ranges: Array<[number, number]>,
    options?: { style?: MarkStyle; color?: MarkColor; message?: string }): Promise<void>;
  clearMarks(id: string): Promise<void>;
}

export function call(method: string, params?: Record<string, unknown>): Promise<unknown>;
export function notify(method: string, params?: Record<string, unknown>): void;
export function activeDocument(): Promise<Document>;
export function documents(): Promise<Document[]>;
export function openFile(path: string): Promise<Document>;
export function newDocument(text?: string, language?: string): Promise<Document>;
export function message(text: string, kind?: 'info' | 'warning' | 'error'): Promise<void>;
export function prompt(text: string, options?: { value?: string; title?: string }): Promise<string | null>;
export function choose<T extends string | { label: string; detail?: string }>(
  items: Iterable<T>, options?: { title?: string; placeholder?: string }): Promise<T | null>;
export function status(id: string, text: string,
  options?: { tooltip?: string; command?: string }): Promise<void>;
export function setClipboard(text: string): Promise<void>;
export function runAction(action: string | Record<string, unknown>): Promise<void>;
export function uiLanguage(): Promise<string>;
export function log(...args: unknown[]): void;
export function setSettings(values: Record<string, unknown>): Promise<Record<string, unknown>>;
export function dataDir(): string;
export function command(title: string, fn: () => unknown,
  options?: { shortcut?: string; id?: string; icon?: string }): Promise<unknown>;
export function transform(title: string, options: Options,
  fn: (text: string, options: Options) => string | Promise<string>,
  extra?: { id?: string; languages?: string | string[] }): Promise<unknown>;
export function on(event: EventName, fn: (arg: any) => unknown): Promise<unknown>;
export function onPanel(panelId: string, fn: (event: {
  item: string; event: string; value: unknown; values: Record<string, unknown>;
}) => unknown): void;
export function showPanel(id: string, options?: {
  title?: string; items?: Item[]; position?: 'right' | 'bottom'; scrollToTop?: boolean;
}): Promise<void>;
export function closePanel(id: string): Promise<void>;
export function run(): void;

export const ui: {
  heading(text: string): Item;
  label(text: string, options?: { weak?: boolean; mono?: boolean }): Item;
  separator(): Item;
  button(id: string, label: string): Item;
  input(id: string, options?: { label?: string; value?: string; hint?: string; multiline?: boolean }): Item;
  checkbox(id: string, label: string, value?: boolean): Item;
  select(id: string, options: string[], extra?: { value?: string; label?: string }): Item;
  row(...items: Item[]): Item;
  tree(id: string, nodes: Item[], options?: { selected?: string }): Item;
  node(label: string, options?: { id?: string; detail?: string; children?: Item[]; open?: boolean }): Item;
  table(id: string, columns: string[], rows: unknown[][]): Item;
  markdown(text: string, options?: { id?: string; base?: string; doc?: Document | number; line?: number }): Item;
};
