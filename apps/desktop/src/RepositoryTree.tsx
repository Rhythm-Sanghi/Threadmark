import { useEffect, useState } from 'react';
import { api } from './api';
import type { TreeEntry } from './api';

export function RepositoryTree({ directory = '', onSelect }: { directory?: string; onSelect: (id: string) => void }) {
  const [entries, setEntries] = useState<TreeEntry[]>([]);
  const [expanded, setExpanded] = useState<string[]>([]);
  const [error, setError] = useState('');
  useEffect(() => { let active = true; api.tree(directory).then(entries => { if (active) setEntries(entries); }).catch(error => { if (active) setError(String(error)); }); return () => { active = false; }; }, [directory]);
  return <ul className="repository-tree" aria-label={directory || 'Repository files'}>{error && <li role="alert">{error}</li>}{entries.map(entry => <li key={entry.path}><button title={entry.path} aria-expanded={entry.directory ? expanded.includes(entry.path) : undefined} onClick={() => { if (entry.directory) setExpanded(current => current.includes(entry.path) ? current.filter(path => path !== entry.path) : [...current, entry.path]); else if (entry.node_id) onSelect(entry.node_id); }}><span className="tree-mark">{entry.directory ? (expanded.includes(entry.path) ? '−' : '+') : '·'}</span>{entry.name}</button>{expanded.includes(entry.path) && <RepositoryTree directory={entry.path} onSelect={onSelect} />}</li>)}{entries.length === 1000 && <li className="hint">First 1,000 entries. Use search for other files.</li>}</ul>;
}
