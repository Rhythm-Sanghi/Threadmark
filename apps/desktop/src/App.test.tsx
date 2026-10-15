import { cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { App } from './App';
import { listen } from '@tauri-apps/api/event';
import { api, getStatus, setTheme } from './api';
import type { Node, Overview, Slice, Status } from './api';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }));
vi.mock('./api', () => ({ getStatus: vi.fn(), setTheme: vi.fn(), api: Object.fromEntries(['recent','chooseRepository','index','select','search','tree','slice','diagnostics','path','cancel','excerpt','configureEditor','openEditor','impact','compare','startWatch','stopWatch','overview'].map(key => [key,vi.fn()])) }));
const status: Status = { schema_version: 1, app_version: '0.1.0', database_schema: 2, database_path: 'C:\\Local\\threadmark.db', theme: 'system', analysis_available: true };
const node: Node = { id: 'work',name: 'work',qualified_name: 'work',kind: 'function',path: 'src/work.ts',language: 'typescript',span: { start_byte: 0,end_byte: 30,line: 1,column: 1,end_line: 2,end_column: 1 },exported: true,test: false };
const other: Node = { ...node,id: 'run',name: 'run',qualified_name: 'run',path: 'src/run.ts' };
const slice: Slice = { schema_version: 1,focus: node.id,nodes: [node,other],edges: [{ id: 'call',from: other.id,to: node.id,kind: 'CALLS',confidence: 'RESOLVED',path: other.path,span: other.span,evidence: 'work()',adapter: 'typescript' }],truncated: false };
const overview: Overview = { schema_version: 1,root: 'C:/repo',name: 'repo',indexed_at: 1,source_files: 2,symbols: 2,relationships: 1,diagnostics: 1,languages: ['typescript'],project_files: [],progress: { phase: 'Done',examined: 2,source_files: 2,parsed_files: 2,reused_files: 0,symbols: 2,relationships: 1,current_path: '' } };
afterEach(cleanup);
beforeEach(() => { vi.resetAllMocks(); vi.mocked(listen).mockResolvedValue(() => {}); vi.mocked(api.cancel).mockResolvedValue(); vi.mocked(api.stopWatch).mockResolvedValue(); vi.mocked(api.startWatch).mockResolvedValue(); vi.mocked(api.overview).mockResolvedValue(overview); vi.mocked(api.impact).mockResolvedValue({ schema_version: 1,targets: [node],direct: [{node: other,hops: 1,evidence_path: slice.edges}],transitive: [],possible: [],related_tests: [],truncated: false,depth_limit: 8 }); vi.mocked(getStatus).mockResolvedValue(status); vi.mocked(api.recent).mockResolvedValue([['C:/repo',1]]); vi.mocked(api.select).mockResolvedValue(overview); vi.mocked(api.tree).mockResolvedValue([{ path: node.path,name: 'work.ts',directory: false,node_id: node.id }]); vi.mocked(api.search).mockResolvedValue([node]); vi.mocked(api.slice).mockResolvedValue(slice); vi.mocked(api.excerpt).mockResolvedValue({ path: node.path,first_line: 1,lines: ['export function work() {}'] }); vi.mocked(api.diagnostics).mockResolvedValue([{ path: 'src/a.ts',line: 2,code: 'unresolved-import',message: 'No target for ./missing' }]); });

async function openRepository() { render(<App />); await userEvent.click(await screen.findByRole('button',{ name: /C:\/repo/ })); }

describe('local explorer', () => {
  it('contains Tab focus in Settings and restores it on close', async () => {
    await openRepository();
    const trigger = screen.getByRole('button', { name: 'Settings' });
    await userEvent.click(trigger);
    const close = screen.getByRole('button', { name: 'Close settings' });
    const editor = screen.getByRole('button', { name: 'Configure editor' });
    close.focus();
    await userEvent.keyboard('{Shift>}{Tab}{/Shift}');
    expect(document.activeElement).toBe(editor);
    await userEvent.tab();
    expect(document.activeElement).toBe(close);
    await userEvent.click(close);
    expect(document.activeElement).toBe(trigger);
  });

  it('navigates node actions with arrows and restores focus after Escape', async () => {
    await openRepository();
    await userEvent.click(await screen.findByRole('button', { name: /work.ts/ }));
    const trigger = await screen.findByRole('button', { name: 'Focus work' });
    trigger.focus();
    await userEvent.keyboard('{Shift>}{F10}{/Shift}');
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Focus here' }));
    await userEvent.keyboard('{ArrowUp}');
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Copy file path' }));
    await userEvent.keyboard('{Home}{ArrowDown}');
    expect(document.activeElement).toBe(screen.getByRole('menuitem', { name: 'Show dependencies' }));
    await userEvent.keyboard('{End}{Escape}');
    expect(screen.queryByRole('menu')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
  it('opens node actions with the keyboard',async()=>{await openRepository();await userEvent.click(await screen.findByRole('button',{name:/work.ts/}));const focused=await screen.findByRole('button',{name:'Focus work'});focused.focus();await userEvent.keyboard('{Shift>}{F10}{/Shift}');expect(await screen.findByRole('menuitem',{name:'Show related tests'})).toBeTruthy();await userEvent.keyboard('{Escape}');expect(screen.queryByRole('menu')).toBeNull();});
  it('reports notification subscription failures',async()=>{vi.mocked(listen).mockRejectedValue('Notifications unavailable');render(<App/>);expect(await screen.findByText('Notifications unavailable')).toBeTruthy();});
  it('starts and stops an explicit watcher',async()=>{await openRepository();await userEvent.click(screen.getByRole('button',{name:'Watch changes'}));expect(api.startWatch).toHaveBeenCalled();await userEvent.click(await screen.findByRole('button',{name:'Stop watching'}));expect(api.stopWatch).toHaveBeenCalled();});
  it('shows impact with inspectable evidence',async()=>{await openRepository();await userEvent.click(await screen.findByRole('button',{name:/work.ts/}));await userEvent.click(await screen.findByRole('button',{name:'Show impact'}));expect(await screen.findByText('Direct · 1')).toBeTruthy();expect(screen.getByRole('button',{name:'run'})).toBeTruthy();expect(screen.getByText('Evidence path')).toBeTruthy();expect(api.impact).toHaveBeenCalledWith('work',8,[]);});
  it('reports a failed Git comparison',async()=>{vi.mocked(api.compare).mockRejectedValue('Git comparison: reference not found');await openRepository();await userEvent.click(screen.getByRole('button',{name:'Changes'}));await userEvent.click(screen.getByRole('button',{name:'Compare'}));expect(await screen.findByText('Git comparison: reference not found')).toBeTruthy();});
  it('persists appearance through Rust and shows real storage', async () => { vi.mocked(setTheme).mockResolvedValue({ ...status,theme: 'dark' }); render(<App />); await screen.findByRole('button',{ name: /C:\/repo/ }); await userEvent.click(screen.getByRole('button',{ name: 'Settings' })); expect(await screen.findByText(status.database_path)).toBeTruthy(); await userEvent.selectOptions(screen.getByLabelText('Color theme'),'dark'); await waitFor(() => expect(document.documentElement.dataset.theme).toBe('dark')); });
  it('searches, focuses a node and exposes relationship evidence', async () => { await openRepository(); await userEvent.type(screen.getByLabelText('Search files and symbols'),'work'); await userEvent.click(await screen.findByRole('button',{ name: /work function/ })); expect(await screen.findByLabelText('Relationship map')).toBeTruthy(); await userEvent.click(screen.getByRole('button',{ name: /CALLS: run to work/ })); expect(await screen.findByText('work()')).toBeTruthy(); expect(screen.getAllByText('RESOLVED').length).toBeGreaterThan(0); });
  it('expands dependents through Rust and navigates history', async () => { await openRepository(); await userEvent.click(await screen.findByRole('button',{ name: /work.ts/ })); await screen.findByLabelText('Relationship map'); await userEvent.click(screen.getByRole('button',{ name: 'Expand dependents' })); await waitFor(() => expect(api.slice).toHaveBeenCalledWith('work','dependents',2)); await userEvent.click(screen.getByRole('button',{ name: 'Focus run' })); await userEvent.click(screen.getByRole('button',{ name: 'Back' })); await waitFor(() => expect(api.slice).toHaveBeenLastCalledWith('work','dependents',2)); });
  it('shows unresolved diagnostics', async () => { await openRepository(); await userEvent.click(screen.getByRole('button',{ name: 'Diagnostics (1)' })); expect(await screen.findByText('No target for ./missing')).toBeTruthy(); await userEvent.click(screen.getByRole('button',{ name: 'Diagnostics (1)' })); expect(screen.queryByText('Reading index diagnostics…')).toBeNull(); });
  it('cancels indexing while the previous repository remains usable', async () => { vi.mocked(api.index).mockImplementation(() => new Promise(() => {})); await openRepository(); await userEvent.click(screen.getByRole('button',{ name: 'Re-index' })); await userEvent.click(await screen.findByRole('button',{ name: 'Cancel indexing' })); expect(api.cancel).toHaveBeenCalled(); });
  it('reports persistence failures without changing saved appearance', async () => { vi.mocked(setTheme).mockRejectedValue('SQLite: locked'); render(<App />); await screen.findByRole('button',{ name: /C:\/repo/ }); await userEvent.click(screen.getByRole('button',{ name: 'Settings' })); await userEvent.selectOptions(await screen.findByLabelText('Color theme'),'dark'); expect(await screen.findByText('SQLite: locked')).toBeTruthy(); expect((screen.getByLabelText('Color theme') as HTMLSelectElement).value).toBe('system'); });
  it('returns keyboard focus to Settings when its dialog closes', async () => {
    render(<App />);
    const trigger = await screen.findByRole('button', { name: 'Settings' });
    await screen.findByRole('button', { name: /C:\/repo/ });
    await userEvent.click(trigger);
    expect(await screen.findByRole('dialog', { name: 'Settings' })).toBeTruthy();
    expect(document.activeElement).toBe(screen.getByLabelText('Color theme'));
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
  it('keeps Ctrl+K and Enter search navigation working', async () => {
    await openRepository();
    await userEvent.keyboard('{Control>}k{/Control}');
    const search = screen.getByRole('combobox', { name: 'Search files and symbols' });
    expect(document.activeElement).toBe(search);
    await userEvent.type(search, 'work');
    await screen.findByRole('listbox', { name: 'Search results' });
    expect(search.getAttribute('aria-activedescendant')).toBe(within(screen.getByRole('listbox')).getByRole('option', { selected: true }).id);
    await userEvent.keyboard('{Enter}');
    expect(await screen.findByLabelText('Relationship map')).toBeTruthy();
    expect(api.slice).toHaveBeenCalledWith('work', 'both', 1);
  });
  it('shows Git changes in a table and retains removed-symbol baseline evidence', async () => {
    const empty = { schema_version: 1, targets: [], direct: [], transitive: [], possible: [], related_tests: [], truncated: false, depth_limit: 8 };
    vi.mocked(api.compare).mockResolvedValue({ schema_version: 1, baseline: 'HEAD', baseline_commit: 'abc123', branch: 'main', changed_files: ['src/work.ts'], symbol_change_count: 2, added_relationship_count: 1, removed_relationship_count: 1, truncated: false, symbol_changes: [{ kind: 'modified', node }, { kind: 'removed', node: other }], added_relationships: [], removed_relationships: slice.edges, current_impact: empty, removed_impact: { ...empty, direct: [{ node: other, hops: 1, evidence_path: slice.edges }] }, diagnostics: [] });
    await openRepository();
    await userEvent.click(screen.getByRole('button', { name: 'Changes' }));
    await userEvent.click(screen.getByRole('button', { name: 'Compare' }));
    expect(await screen.findByRole('table')).toBeTruthy();
    expect(screen.getAllByRole('button', { name: 'Inspect' })).toHaveLength(1);
    expect(screen.getByText('Baseline only')).toBeTruthy();
    await userEvent.click(screen.getByText('Baseline evidence'));
    expect(screen.getByText(/work\(\)/)).toBeTruthy();
  });

});





