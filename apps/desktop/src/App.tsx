import { EmptyState, PageHeading, SectionHeading } from './ui';
import { ChangesPanel } from './ChangesPanel';
import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { api, getStatus, setTheme } from './api';
import type { Diagnostic, Edge, Excerpt, Node, Overview, Progress, Slice, Status, Theme } from './api';
import { Canvas } from './Canvas';
import { RepositoryTree } from './RepositoryTree';
import { ImpactPanel } from './ImpactPanel';

function message(error: unknown): string { return error instanceof Error ? error.message : String(error); }

export function App() {
  const [status, setStatus] = useState<Status>();
  const [overview, setOverview] = useState<Overview>();
  const [recent, setRecent] = useState<[string, number][]>([]);
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);
  const [indexing, setIndexing] = useState(false);
  const [watching,setWatching]=useState(false);
  const [revision,setRevision]=useState(0);
  const [context,setContext]=useState<{node:Node;x:number;y:number}>();
  const [progress, setProgress] = useState<Progress>();
  const [settings, setSettings] = useState(false);
  const settingsTrigger = useRef<HTMLButtonElement>(null);
  const preferences = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!settings) return;
    preferences.current?.querySelector<HTMLSelectElement>('select')?.focus();
    const containFocus = (event: KeyboardEvent) => {
      if (event.key !== 'Tab') return;
      const controls = Array.from(preferences.current?.querySelectorAll<HTMLElement>('button:not(:disabled), select:not(:disabled), [href], input:not(:disabled), [tabindex="0"]') ?? []);
      const index = controls.indexOf(document.activeElement as HTMLElement);
      if (controls.length && (index < 0 || (event.shiftKey ? index === 0 : index === controls.length - 1))) {
        event.preventDefault();
        controls[event.shiftKey ? controls.length - 1 : 0].focus();
      }
    };
    window.addEventListener('keydown', containFocus);
    return () => window.removeEventListener('keydown', containFocus);
  }, [settings]);
  function closeSettings() { setSettings(false); settingsTrigger.current?.focus(); }
  const [filesOpen, setFilesOpen] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<Node[]>([]);
  const [activeResult, setActiveResult] = useState(0);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [slice, setSlice] = useState<Slice>();
  const [evidence, setEvidence] = useState<Edge>();
  const [direction, setDirection] = useState('both');
  const [depth, setDepth] = useState(1);
  const [tab, setTab] = useState('Map');
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
  const [diagnosticsLoading, setDiagnosticsLoading] = useState(false);
  const [pathFrom, setPathFrom] = useState('');
  const [pathTo, setPathTo] = useState('');
  const [excerpt, setExcerpt] = useState<Excerpt>();
  const search = useRef<HTMLInputElement>(null);
  const contextMenu = useRef<HTMLDivElement>(null);
  const focus = slice?.nodes.find(node => node.id === slice.focus);
  const focusId = history[historyIndex];
  useEffect(() => {
    if (!context) return;
    const trigger = document.activeElement as HTMLElement | null;
    let restoreFocus = false;
    const close = () => setContext(undefined);
    const onKey = (event: KeyboardEvent) => {
      const items = Array.from(contextMenu.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? []);
      const index = items.indexOf(document.activeElement as HTMLButtonElement);
      let next: number | undefined;
      if (event.key === 'ArrowDown') next = (index + 1) % items.length;
      if (event.key === 'ArrowUp') next = (index - 1 + items.length) % items.length;
      if (event.key === 'Home') next = 0;
      if (event.key === 'End') next = items.length - 1;
      if (next !== undefined && items[next]) { event.preventDefault(); items[next].focus(); }
      if (event.key === 'Escape') { event.preventDefault(); event.stopImmediatePropagation(); restoreFocus = true; close(); }
      if (event.key === 'Tab') close();
    };
    contextMenu.current?.querySelector<HTMLButtonElement>('button')?.focus();
    window.addEventListener('click', close);
    window.addEventListener('keydown', onKey, true);
    return () => {
      window.removeEventListener('click', close);
      window.removeEventListener('keydown', onKey, true);
      if (restoreFocus && trigger?.isConnected) trigger.focus();
    };
  }, [context]);

  async function load() { setBusy(true); setError(''); try { setStatus(await getStatus()); setRecent(await api.recent()); } catch (error) { setError(message(error)); } finally { setBusy(false); } }
  useEffect(()=>{let active=true;const subscriptions=[listen<Overview>('index-updated',event=>{if(active){setOverview(event.payload);setRevision(value=>value+1);setEvidence(undefined);}}),listen<string>('watch-error',event=>{if(active)setError(event.payload);}),listen<boolean>('watch-status',event=>{if(active)setWatching(event.payload);})].map(subscription=>subscription.catch(error=>{if(active)setError(message(error));return()=>{};}));return()=>{active=false;for(const subscription of subscriptions)void subscription.then(stop=>stop()).catch(()=>{});};},[]);
  async function toggleWatch(){try{if(watching){await api.stopWatch();setWatching(false);}else{await api.startWatch();setWatching(true);}}catch(error){setError(message(error));}}
  useEffect(() => {
    let active = true;
    getStatus().then(status => { if (active) setStatus(status); }).catch(error => { if (active) setError(message(error)); });
    api.recent().then(recent => { if (active) setRecent(recent); }).catch(error => { if (active) setError(message(error)); });
    const onKey = (event: KeyboardEvent) => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') { event.preventDefault(); search.current?.focus(); } if (event.key === 'Escape') { setQuery(''); setEvidence(undefined); if (settingsTrigger.current?.getAttribute('aria-expanded') === 'true') settingsTrigger.current.focus(); setSettings(false); setFilesOpen(false); setInspectorOpen(false); } };
    window.addEventListener('keydown', onKey);
    return () => { active = false; window.removeEventListener('keydown', onKey); };
  }, []);
  useEffect(() => { if (status) document.documentElement.dataset.theme = status.theme; }, [status]);
  useEffect(() => { let active = true; const unlisten = listen<Progress>('index-progress', event => { if (active) setProgress(event.payload); }).catch(error => { if(active)setError(message(error)); return undefined; }); return () => { active = false; void unlisten.then(stop => stop?.()); }; }, []);
  useEffect(() => { if (!overview || !query) { setResults([]); return; } let active = true; const timer = setTimeout(() => { api.search(query).then(results => { if (active) { setResults(results); setActiveResult(0); } }).catch(error => { if (active) setError(message(error)); }); }, 100); return () => { active = false; clearTimeout(timer); }; }, [query, overview]);
  useEffect(() => { if (!focusId || !overview) return; let active = true; api.slice(focusId, direction, depth).then(slice => { if (active) { setSlice(slice); setEvidence(undefined); } }).catch(error => { if (active) setError(message(error)); }); return () => { active = false; }; }, [focusId, direction, depth, overview]);
  useEffect(() => { if (tab !== 'Diagnostics' || !overview) return; let active = true; setDiagnosticsLoading(true); api.diagnostics().then(items => { if (active) setDiagnostics(items); }).catch(error => { if (active) setError(message(error)); }).finally(() => { if (active) setDiagnosticsLoading(false); }); return () => { active = false; }; }, [tab, overview]);
  useEffect(() => { if (!focus) { setExcerpt(undefined); return; } let active = true; api.excerpt(focus.id).then(excerpt => { if (active) setExcerpt(excerpt); }).catch(error => { if (active) { setExcerpt(undefined); setError(message(error)); } }); return () => { active = false; }; }, [focus]);
  async function changeTheme(theme: Theme) { setBusy(true); setError(''); try { setStatus(await setTheme(theme)); } catch (error) { setError(message(error)); } finally { setBusy(false); } }
  function navigate(id: string) { setHistory(current => [...current.slice(0, historyIndex + 1), id]); setHistoryIndex(historyIndex + 1); setQuery(''); setTab('Map'); setFilesOpen(false); }
  function repositoryLoaded(value: Overview) { setOverview(value); setRevision(current=>current+1); setHistory([]); setHistoryIndex(-1); setSlice(undefined); setEvidence(undefined); setTab('Map'); setQuery(''); }
  async function open(path?: string) { try { const selected = path ?? await api.chooseRepository(); if (!selected) return; await api.stopWatch(); setWatching(false); setBusy(true); setIndexing(true); setError(''); setProgress(undefined); repositoryLoaded(await api.index(selected)); setRecent(await api.recent()); } catch (error) { setError(message(error)); } finally { setBusy(false); setIndexing(false); setProgress(undefined); } }
  async function openRecent(path: string) { setError(''); try { await api.stopWatch(); setWatching(false); repositoryLoaded(await api.select(path)); } catch (error) { setError(message(error)); } }
  async function trace() { try { setSlice(await api.path(pathFrom, pathTo)); setEvidence(undefined); } catch (error) { setError(message(error)); } }

  return <div className={`application ${overview ? 'explorer' : ''}`}>
    <header className="app-header"><img src="/threadmark.svg" width="28" height="28" alt="" /><strong>Threadmark</strong>{overview && <span className="repository-name" title={overview.root}>{overview.name}</span>}
      {overview && <div className="search"><label htmlFor="repository-search">Search</label><input id="repository-search" ref={search} role="combobox" aria-autocomplete="list" aria-activedescendant={results[activeResult] ? `search-result-${activeResult}` : undefined} aria-expanded={results.length > 0} aria-controls={results.length > 0 ? "search-results" : undefined} aria-label="Search files and symbols"  placeholder="Files and symbols" value={query} onChange={event => setQuery(event.target.value)} onKeyDown={event => { if (event.key === 'ArrowDown') { event.preventDefault(); setActiveResult(index => Math.min(results.length - 1, index + 1)); } if (event.key === 'ArrowUp') { event.preventDefault(); setActiveResult(index => Math.max(0, index - 1)); } if (event.key === 'Enter' && results[activeResult]) navigate(results[activeResult].id); }} /><kbd aria-hidden="true">Ctrl+K</kbd>{results.length > 0 && <ul id="search-results" className="search-results" role="listbox" aria-label="Search results">{results.map((node, index) => <li id={`search-result-${index}`} key={node.id} role="option" aria-selected={index === activeResult}><button aria-label={`${node.name} ${node.kind} ${node.path}:${node.span.line}`} onClick={() => navigate(node.id)}><strong>{node.name}</strong><span>{node.kind} · {node.path}:{node.span.line}</span></button></li>)}</ul>}</div>}
      <div className="header-actions"><button disabled={busy || !status} onClick={() => void open()}>Open repository</button><button ref={settingsTrigger} aria-controls="preferences" aria-expanded={settings} onClick={() => setSettings(value => !value)}>Settings</button></div>
    </header>
    {error && <div className="error-bar" role="alert">{error}<button onClick={() => setError('')}>Dismiss</button>{!status && <button disabled={busy} onClick={() => void load()}>Retry</button>}</div>}
    {settings && status && <div ref={preferences} id="preferences" className="preferences" role="dialog" aria-labelledby="settings-title"><div className="preferences-header"><h2 id="settings-title">Settings</h2><button className="quiet" onClick={closeSettings}>Close settings</button></div><div className="setting"><label htmlFor="theme">Color theme</label><select id="theme" value={status.theme} disabled={busy} onChange={event => void changeTheme(event.target.value as Theme)}><option value="system">Use system setting</option><option value="light">Light</option><option value="dark">Dark</option></select></div><div className="setting-block"><SectionHeading>Editor</SectionHeading><button onClick={() => void api.configureEditor().catch(error => setError(message(error)))}>Configure editor</button><p className="hint">Choose an installed executable. Custom editor arguments are available through the CLI.</p></div><div className="setting-block"><SectionHeading>Local storage</SectionHeading><p className="path">{status.database_path}</p><p className="hint">This unencrypted database stores settings, source evidence and cached syntax. It is shared with the CLI.</p></div></div>}
    {indexing && <div className="index-progress" role="status"><strong>{progress?.phase ?? 'Opening repository…'}</strong><span>{progress?.examined ?? 0} files examined · {progress?.symbols ?? 0} symbols · {progress?.relationships ?? 0} relationships</span><span className="path">{progress?.current_path}</span><button onClick={() => void api.cancel().catch(error => setError(message(error)))}>Cancel indexing</button></div>}
    {!overview ? <main className="welcome"><div className="welcome-layout"><div className="welcome-rail">LOCAL / STATIC<br />JS · TS · Python</div><div><PageHeading title="Open a codebase." description="Trace dependencies, inspect the evidence, and check what a change could reach." /><p className="intro">Choose a local repository. Threadmark reads source files without running the project.</p><button className="primary" disabled={busy || !status} onClick={() => void open()}>Choose repository folder</button>{!status && !error && <p role="status">Opening local storage…</p>}{recent.length > 0 ? <section><SectionHeading index="01" meta={`${recent.length} saved`}>Recent repositories</SectionHeading><ul className="recent">{recent.map(([path, timestamp]) => <li key={path}><button disabled={busy} onClick={() => void openRecent(path)}><strong>{path.replaceAll('\\', '/').split('/').pop()}</strong><time dateTime={new Date(timestamp * 1000).toISOString()}>{new Date(timestamp * 1000).toLocaleDateString()}</time><span className="path">{path}</span></button></li>)}</ul></section> : status && <section><EmptyState title="No recent repositories.">Open a folder to build your first local index.</EmptyState></section>}<p className="hint">No account. No telemetry. No repository scripts. Static reachability does not predict runtime behavior or guarantee test coverage.</p></div></div></main> : <>
      <nav className="workspace-tabs" aria-label="Workspace">{['Map', 'Impact', 'Changes', 'Tests', 'Diagnostics'].map(name => <button key={name} aria-current={tab === name ? 'page' : undefined} onClick={() => { if (name === 'Diagnostics' && tab !== name) setDiagnosticsLoading(true); setTab(name); }}>{name}{name === 'Diagnostics' ? ` (${overview.diagnostics})` : ''}</button>)}<span className="workspace-meta">{overview.source_files} source files · {overview.symbols} symbols</span><div className="workspace-actions"><button className="pane-toggle" aria-expanded={filesOpen} aria-controls="repository-pane" onClick={() => { setFilesOpen(value => !value); setInspectorOpen(false); }}>Files</button><button className="pane-toggle" aria-expanded={inspectorOpen} aria-controls="inspector-pane" onClick={() => { setInspectorOpen(value => !value); setFilesOpen(false); }}>Inspector</button><button disabled={busy} aria-pressed={watching} onClick={()=>void toggleWatch()}>{watching ? 'Stop watching' : 'Watch changes'}</button><button disabled={busy} onClick={() => void open(overview.root)}>Re-index</button></div></nav>
      <div className="workspace"><aside id="repository-pane" className={`tree-pane ${filesOpen ? "drawer-open" : ""}`}><SectionHeading index="01">Repository</SectionHeading><RepositoryTree key={`${overview.root}:${overview.indexed_at}:${revision}`} onSelect={navigate} /><section><button onClick={()=>void api.exportMetadata().catch(error=>setError(message(error)))}>Export metadata JSON</button><SectionHeading>Detected configuration</SectionHeading>{overview.project_files.slice(0,20).map(path => <p className="path" key={path}>{path}</p>)}</section></aside>
        <div className="working-pane">{tab === 'Changes' ? <ChangesPanel onBusy={setBusy} onRefresh={value=>{setOverview(current=>current?.root===value.root?value:current);setRevision(current=>current+1);}} onNode={navigate} onEdge={setEvidence} onError={setError} /> : (tab === 'Impact' || tab === 'Tests') ? <ImpactPanel testsOnly={tab === 'Tests'} selector={focusId} onNode={navigate} onEdge={setEvidence} onError={setError} /> : tab === 'Diagnostics' ? <div className="diagnostics"><PageHeading title="Analysis diagnostics" description="Unresolved source and configuration. Showing up to 2,000 diagnostics." />{diagnosticsLoading ? <p role="status">Reading index diagnostics…</p> : diagnostics.length === 0 && <EmptyState title="No diagnostics in this index." />}{diagnostics.map((item, index) => <article key={index}><div><strong>{item.code}</strong><p className="path">{item.path}{item.line ? `:${item.line}` : ''}</p></div><p>{item.message}</p></article>)}</div> : <>
          <div className="map-navigation"><button aria-label="Back" disabled={historyIndex < 1} onClick={() => setHistoryIndex(index => index - 1)}>← Back</button><button aria-label="Forward" disabled={historyIndex >= history.length - 1} onClick={() => setHistoryIndex(index => index + 1)}>Forward →</button><label>Explore <select name="map-direction" value={direction} onChange={event => setDirection(event.target.value)}><option value="both">Both directions</option><option value="dependencies">Dependencies</option><option value="dependents">Dependents</option></select></label><label>Hops <select name="map-depth" value={depth} onChange={event => setDepth(Number(event.target.value))}>{[1,2,3].map(value => <option key={value}>{value}</option>)}</select></label></div>
          {slice ? <Canvas key={slice.focus} onContext={(node,x,y)=>setContext({node,x,y})} slice={slice} onNode={node => navigate(node.id)} onEdge={setEvidence} /> : <div className="empty-map"><SectionHeading index="02">Relationship map</SectionHeading><h1>{overview.name}</h1><EmptyState title="Select a file or search for a symbol.">Start with one neighborhood. Follow its edges to inspect dependencies and their evidence.</EmptyState></div>}
          <form className="path-form" onSubmit={event => { event.preventDefault(); void trace(); }}><label>From<input name="path-from" value={pathFrom} onChange={event => setPathFrom(event.target.value)} placeholder="File, symbol or ID" /></label><label>To<input name="path-to" value={pathTo} onChange={event => setPathTo(event.target.value)} placeholder="File, symbol or ID" /></label><button disabled={!pathFrom || !pathTo}>Find path</button></form>
        </>}</div>
        <aside id="inspector-pane" className={`inspector ${inspectorOpen ? "drawer-open" : ""}`}><SectionHeading index="03">Inspector</SectionHeading>{evidence ? <><h3>{evidence.kind}</h3><dl><dt>Confidence</dt><dd>{evidence.confidence}</dd><dt>Adapter</dt><dd>{evidence.adapter}</dd></dl><p className="path">{evidence.path}:{evidence.span.line}:{evidence.span.column}</p><pre>{evidence.evidence}</pre><button onClick={() => navigate(evidence.from)}>Focus source</button><button onClick={() => navigate(evidence.to)}>Focus target</button></> : focus ? <><h3>{focus.qualified_name}</h3><p className="hint">{focus.kind} · {focus.language}{focus.exported ? ' · exported' : ''}</p><p className="path">{focus.path}:{focus.span.line}:{focus.span.column}</p><div className="inspector-actions"><button className="primary" onClick={() => setTab('Impact')}>Show impact</button><button onClick={() => void api.openEditor(focus.id).catch(error => setError(message(error)))}>Open in editor</button><button onClick={() => { setDirection('dependencies'); setDepth(value => Math.min(3, value + 1)); }}>Expand dependencies</button><button onClick={() => { setDirection('dependents'); setDepth(value => Math.min(3, value + 1)); }}>Expand dependents</button><button onClick={() => setDepth(1)}>Collapse branches</button><button onClick={() => void navigator.clipboard.writeText(focus.qualified_name).catch(error => setError(message(error)))}>Copy name</button><button onClick={() => void navigator.clipboard.writeText(focus.path).catch(error => setError(message(error)))}>Copy path</button></div><section><SectionHeading>Source preview</SectionHeading>{excerpt?.stale && <p role="status" className="hint">Source has changed since indexing. Re-index to refresh spans and relationships.</p>}{excerpt && <pre>{excerpt.lines.map((line, index) => `${excerpt.first_line + index} │ ${line}`).join("\n")}</pre>}</section><section><SectionHeading>Relationships in view</SectionHeading><ul className="relationship-list">{slice?.edges.filter(edge => edge.from === focus.id || edge.to === focus.id).map(edge => <li key={edge.id}><button onClick={() => setEvidence(edge)}><span>{edge.kind} · {edge.confidence}</span><strong>{slice.nodes.find(node => node.id === (edge.from === focus.id ? edge.to : edge.from))?.name}</strong></button></li>)}</ul></section></> : <p className="hint">Select a node or relationship to inspect its evidence.</p>}</aside>
      </div>
    </>}
    {context && <div ref={contextMenu} role="menu" className="node-menu" style={{left:Math.min(context.x,window.innerWidth-230),top:Math.max(8,Math.min(context.y,window.innerHeight-370))}}>{['Focus here','Show dependencies','Show dependents','Show impact','Show related tests','Find path from','Find path to','Open in editor','Copy qualified name','Copy file path'].map(action=><button role="menuitem" key={action} onClick={()=>{const node=context.node;setContext(undefined);if(action.startsWith('Copy')){void navigator.clipboard.writeText(action==='Copy file path'?node.path:node.qualified_name).catch(error=>setError(message(error)));return;}if(action==='Open in editor'){void api.openEditor(node.id).catch(error=>setError(message(error)));return;}navigate(node.id);if(action==='Show dependencies')setDirection('dependencies');if(action==='Show dependents')setDirection('dependents');if(action==='Show impact')setTab('Impact');if(action==='Show related tests')setTab('Tests');if(action==='Find path from')setPathFrom(node.id);if(action==='Find path to')setPathTo(node.id);}}>{action}</button>)}</div>}
    <footer><span className="local-mark" aria-hidden="true" />Local analysis<span className="footer-meta">{overview ? `${overview.languages.join(' · ')} · Index saved ${new Date(overview.indexed_at * 1000).toLocaleString()}` : 'No account. No telemetry.'}</span>{status && <span className="version">v{status.app_version}</span>}</footer>
  </div>;
}









