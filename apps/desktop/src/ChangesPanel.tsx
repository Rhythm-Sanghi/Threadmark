import { useState } from 'react';
import { api } from './api';
import type { Comparison, Edge, Overview } from './api';
import { ImpactRows } from './ImpactPanel';
import { EmptyState, PageHeading, SectionHeading } from './ui';

export function ChangesPanel({ onNode, onEdge, onError, onRefresh, onBusy }: { onNode: (id: string) => void; onEdge: (edge: Edge) => void; onError: (message: string) => void; onRefresh: (overview: Overview) => void; onBusy: (busy: boolean) => void }) {
  const [against, setAgainst] = useState('HEAD');
  const [result, setResult] = useState<Comparison>();
  const [busy, setBusy] = useState(false);
  async function compare() {
    setBusy(true); onBusy(true); setResult(undefined);
    try { await api.stopWatch(); setResult(await api.compare(against)); onRefresh(await api.overview()); }
    catch (error) { onError(String(error)); }
    finally { setBusy(false); onBusy(false); }
  }
  return <div className="analysis-panel">
    <PageHeading title="Working-tree changes" description="Compare local source with a Git reference. Includes unstaged and untracked source files." />
    <form className="analysis-controls" onSubmit={event => { event.preventDefault(); void compare(); }}>
      <label>Baseline reference<input name="baseline-reference" value={against} onChange={event => setAgainst(event.target.value)} required maxLength={256} /></label>
      <button className="primary" disabled={busy}>{busy ? 'Comparing…' : 'Compare'}</button>
    </form>
    {busy && <p role="status">Refreshing source and reading local Git objects…</p>}
    {!result && !busy && <EmptyState title="Choose a baseline, then compare.">HEAD compares with the latest local commit. You can also use a local branch or commit hash.</EmptyState>}
    {result && <>
      <p className="path">{result.branch} · {result.baseline} · {result.baseline_commit}</p>
      {result.truncated && <p role="status" className="hint">Comparison or impact limit reached. Lists are incomplete; change totals include omitted entries.</p>}
      <section><SectionHeading index="01" meta={`${result.changed_files.length} files`}>Changed source files</SectionHeading>
        {result.changed_files.length === 0 ? <EmptyState title="No source changes against this baseline." /> : <ul className="changed-files">{result.changed_files.map(path => <li className="path" key={path}>{path}</li>)}</ul>}
      </section>
      <section><SectionHeading index="02" meta={result.symbol_change_count}>Symbol changes</SectionHeading>
        <div className="table-scroll"><table><thead><tr><th scope="col">Change</th><th scope="col">Symbol / source</th><th scope="col">Action</th></tr></thead><tbody>
          {result.symbol_changes.filter(change => change.node.kind !== 'file').map(change => <tr key={change.node.id}><td><span className="status-text">{change.kind}</span></td><td><code>{change.node.qualified_name}</code><p className="path">{change.node.path}:{change.node.span.line}</p></td><td>{change.kind !== 'removed' ? <button onClick={() => onNode(change.node.id)}>Inspect</button> : <span className="hint">Baseline only</span>}</td></tr>)}
        </tbody></table></div>
      </section>
      <section><SectionHeading index="03">Relationship changes</SectionHeading><p className="path">{result.added_relationship_count} added · {result.removed_relationship_count} removed</p></section>
      <ImpactRows title="Current direct impact" items={result.current_impact.direct} onNode={onNode} onEdge={onEdge} />
      <ImpactRows title="Current transitive impact" items={result.current_impact.transitive} onNode={onNode} onEdge={onEdge} />
      <section className="impact-group"><SectionHeading>Baseline impact of removed symbols</SectionHeading><p className="hint">Baseline relationships can include source that no longer exists.</p>
        {[...result.removed_impact.direct, ...result.removed_impact.transitive].map(item => <article key={item.node.id}><code>{item.node.qualified_name}</code><p className="path">{item.node.path}:{item.node.span.line}</p><details><summary>Baseline evidence</summary>{item.evidence_path.map(edge => <pre key={edge.id}>{edge.kind} · {edge.path}:{edge.span.line}{'\n'}{edge.evidence}</pre>)}</details></article>)}
      </section>
    </>}
  </div>;
}
