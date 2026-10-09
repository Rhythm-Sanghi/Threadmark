import { invoke, isTauri } from '@tauri-apps/api/core';

export type Theme = 'system' | 'light' | 'dark';

export interface Status {
  schema_version: number;
  app_version: string;
  database_schema: number;
  database_path: string;
  theme: Theme;
  analysis_available: boolean;
}

export function getStatus(): Promise<Status> {
  if (!isTauri()) return Promise.reject(new Error('Local storage is available in the desktop app. Start Threadmark with npm run tauri dev.'));
  return invoke<Status>('get_status');
}

export function setTheme(theme: Theme): Promise<Status> {
  return invoke<Status>('set_theme', { theme });
}

export interface Span { start_byte: number; end_byte: number; line: number; column: number; end_line: number; end_column: number }
export interface Node { id: string; name: string; qualified_name: string; kind: string; path: string; language: string; span: Span; exported: boolean; test: boolean }
export interface Edge { id: string; from: string; to: string; kind: string; confidence: string; path: string; span: Span; evidence: string; adapter: string }
export interface Slice { schema_version: number; focus: string; nodes: Node[]; edges: Edge[]; truncated: boolean }
export interface Progress { phase: string; examined: number; source_files: number; parsed_files: number; reused_files: number; symbols: number; relationships: number; current_path: string }
export interface Overview { schema_version: number; root: string; name: string; indexed_at: number; source_files: number; symbols: number; relationships: number; diagnostics: number; languages: string[]; project_files: string[]; progress: Progress }
export interface Diagnostic { path: string; line: number; code: string; message: string }
export interface TreeEntry { path: string; name: string; directory: boolean; node_id: string | null }
export interface Excerpt { path: string; first_line: number; lines: string[]; stale?:boolean }
export interface ImpactItem { node: Node; hops: number; evidence_path: Edge[] }
export interface Impact { schema_version: number; targets: Node[]; direct: ImpactItem[]; transitive: ImpactItem[]; possible: ImpactItem[]; related_tests: ImpactItem[]; truncated: boolean; depth_limit: number }
export interface Comparison { symbol_change_count:number;added_relationship_count:number;removed_relationship_count:number;truncated:boolean;schema_version:number;baseline:string;baseline_commit:string;branch:string;changed_files:string[];symbol_changes:{kind:string;node:Node}[];added_relationships:Edge[];removed_relationships:Edge[];current_impact:Impact;removed_impact:Impact;diagnostics:Diagnostic[] }
export const api = {
  exportMetadata:()=>invoke<string|null>('export_metadata'),
  startWatch:()=>invoke<void>('start_watch'),
  stopWatch:()=>invoke<void>('stop_watch'),
  compare: (against:string)=>invoke<Comparison>('compare_changes',{against}),
  chooseRepository: () => invoke<string | null>('choose_repository'),
  recent: () => invoke<[string,number][]>('recent_repositories'),
  overview: () => invoke<Overview>('overview'),
  select: (path: string) => invoke<Overview>('select_repository',{path}),
  index: (path: string) => invoke<Overview>('index_repository',{path}),
  cancel: () => invoke<void>('cancel_index'),
  search: (query: string) => invoke<Node[]>('search',{query}),
  tree: (directory: string) => invoke<TreeEntry[]>('repository_tree',{directory}),
  slice: (selector: string,direction = 'both',depth = 1,kinds: string[] = []) => invoke<Slice>('graph_slice',{selector,direction,depth,kinds}),
  diagnostics: () => invoke<Diagnostic[]>('diagnostics'),
  path: (from: string,to: string) => invoke<Slice>('find_path',{from,to}),
  excerpt: (selector: string) => invoke<Excerpt>('source_excerpt',{selector}),
  openEditor: (selector: string) => invoke<void>('open_in_editor',{selector}),
  configureEditor: () => invoke<string | null>('configure_editor'),
  impact: (selector: string,depth = 8,kinds: string[] = []) => invoke<Impact>('analyze_impact',{selector,depth,kinds}),
};


