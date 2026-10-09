import { useEffect, useRef, useState } from 'react';
import type { Edge, Node, Slice } from './api';

const NODE_WIDTH = 216;
const NODE_HEIGHT = 68;
function compactName(name: string) {
  return name.length > 25 ? `${name.slice(0, 14)}…${name.slice(-10)}` : name;
}

export function Canvas({ slice, onNode, onEdge, onContext }: { slice: Slice; onNode: (node: Node) => void; onEdge: (edge: Edge) => void; onContext?: (node: Node, x: number, y: number) => void }) {
  const [view, setView] = useState({ x: 0, y: 0, zoom: 1 });
  const [drag, setDrag] = useState<{ x: number; y: number }>();
  const [list, setList] = useState(false);
  const [viewport, setViewport] = useState({ width: 840, height: 500 });
  const viewportRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const element = viewportRef.current;
    if (!element || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(entries => {
      const rectangle = entries[0]?.contentRect;
      if (rectangle && rectangle.width > 0 && rectangle.height > 0) {
        const next = { width: Math.round(rectangle.width), height: Math.round(rectangle.height) };
        setViewport(current => current.width === next.width && current.height === next.height ? current : next);
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const nodes = new Map(slice.nodes.map(node => [node.id, node]));
  const incoming = new Set(slice.edges.filter(edge => edge.to === slice.focus).map(edge => edge.from));
  const others = slice.nodes.filter(node => node.id !== slice.focus);
  const left = others.filter(node => incoming.has(node.id));
  const right = others.filter(node => !incoming.has(node.id));
  const height = Math.max(500, Math.max(left.length, right.length) * 96 + 100);
  const positions = new Map<string, { x: number; y: number }>([[slice.focus, { x: 420, y: height / 2 }]]);
  left.forEach((node, index) => positions.set(node.id, { x: 135, y: (index + 1) * height / (left.length + 1) }));
  right.forEach((node, index) => positions.set(node.id, { x: 705, y: (index + 1) * height / (right.length + 1) }));
  const baseScale = Math.min(1, viewport.width / 840);
  const scale = baseScale * view.zoom;
  function zoom(factor: number, point = { x: 0, y: 0 }) {
    setView(current => {
      const next = Math.max(.12, Math.min(3 / baseScale, current.zoom * factor));
      const ratio = next / current.zoom;
      return { zoom: next, x: point.x - (point.x - current.x) * ratio, y: point.y - (point.y - current.y) * ratio };
    });
  }
  function contextKey(event: React.KeyboardEvent, node: Node) {
    if (event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10')) {
      event.preventDefault();
      const bounds = event.currentTarget.getBoundingClientRect();
      onContext?.(node, bounds.x + bounds.width / 2, bounds.y + bounds.height / 2);
    }
  }
  return <div className="canvas">
    <div className="canvas-tools">
      {!list && <><button onClick={() => zoom(1.2)}>Zoom in</button><button onClick={() => zoom(1 / 1.2)}>Zoom out</button><button onClick={() => setView({ x: 0, y: 0, zoom: Math.min(1, viewport.width / 840, viewport.height / height) / baseScale })}>Fit selection</button><button onClick={() => setView({ x: 0, y: 0, zoom: 1 / baseScale })}>Actual size</button></>}
      <button aria-pressed={list} onClick={() => setList(value => !value)}>{list ? 'Map view' : 'List view'}</button>
      <span>{!list && `${Math.round(scale * 100)}% · `}{slice.nodes.length} nodes{slice.truncated ? ' · limit reached' : ''}</span>
    </div>
    <div ref={viewportRef} className="map-viewport">
      {list ? <div className="map-list"><table><caption>Nodes in this neighborhood</caption><thead><tr><th scope="col">Kind</th><th scope="col">Name / source</th></tr></thead><tbody>{slice.nodes.map(node => <tr key={node.id} aria-current={node.id === slice.focus ? 'true' : undefined}><td className="status-text">{node.kind}</td><td><button className="text-button" onClick={() => onNode(node)} onContextMenu={event => { event.preventDefault(); onContext?.(node, event.clientX, event.clientY); }} onKeyDown={event => contextKey(event, node)}>{node.qualified_name}</button><p className="path">{node.path}:{node.span.line}</p></td></tr>)}</tbody></table></div> : <svg aria-label="Relationship map" viewBox={`0 0 ${viewport.width} ${viewport.height}`}
        onPointerDown={event => { if (event.target === event.currentTarget) { event.currentTarget.setPointerCapture(event.pointerId); setDrag({ x: event.clientX, y: event.clientY }); } }}
        onPointerMove={event => { if (drag) { setView(current => ({ ...current, x: current.x + event.clientX - drag.x, y: current.y + event.clientY - drag.y })); setDrag({ x: event.clientX, y: event.clientY }); } }}
        onPointerUp={() => setDrag(undefined)} onPointerCancel={() => setDrag(undefined)}
        onWheel={event => { const bounds = event.currentTarget.getBoundingClientRect(); zoom(event.deltaY < 0 ? 1.08 : .92, { x: event.clientX - bounds.left - viewport.width / 2, y: event.clientY - bounds.top - viewport.height / 2 }); }}>
        <defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="currentColor" /></marker></defs>
        <g transform={`translate(${viewport.width / 2 + view.x - 420 * scale} ${viewport.height / 2 + view.y - height / 2 * scale}) scale(${scale})`}>
          {slice.edges.map(edge => {
            const from = positions.get(edge.from), to = positions.get(edge.to);
            if (!from || !to) return null;
            const sign = to.x >= from.x ? 1 : -1, start = from.x + sign * 108, end = to.x - sign * 112;
            const d = `M${start} ${from.y} C${(start + end) / 2} ${from.y}, ${(start + end) / 2} ${to.y}, ${end} ${to.y}`;
            return <g key={edge.id} className="map-edge" role="button" tabIndex={0} aria-label={`${edge.kind}: ${nodes.get(edge.from)?.name} to ${nodes.get(edge.to)?.name}`} onClick={() => onEdge(edge)} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); onEdge(edge); } }}><title>{edge.kind} · {edge.confidence}</title><path className="edge-hit" d={d} /><path className={edge.confidence === 'INFERRED' ? 'inferred' : ''} d={d} markerEnd="url(#arrow)" />{scale >= .6 && <text x={(start + end) / 2} y={(from.y + to.y) / 2 - 8} textAnchor="middle">{edge.kind.toLowerCase().replaceAll('_', ' ')}</text>}</g>;
          })}
          {slice.nodes.map(node => {
            const position = positions.get(node.id)!;
            return <g key={node.id} transform={`translate(${position.x - NODE_WIDTH / 2} ${position.y - NODE_HEIGHT / 2})`} role="button" tabIndex={0} aria-label={`Focus ${node.name}`} className={`map-node ${node.id === slice.focus ? 'focused' : ''}`} onContextMenu={event => { event.preventDefault(); onContext?.(node, event.clientX, event.clientY); }} onClick={() => onNode(node)} onKeyDown={event => { contextKey(event, node); if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); onNode(node); } }}><title>{node.qualified_name} — {node.path}:{node.span.line}</title><rect width={NODE_WIDTH} height={NODE_HEIGHT} rx={node.kind === 'file' ? '0' : '4'} /><path d={node.kind === 'class' ? 'M10 9h8v8h-8z' : node.kind === 'file' ? 'M10 9h7l3 3v7H10z' : 'M11 13h8M15 9v8'} /><text className="node-kind" x="26" y="18">{node.kind.toUpperCase()}</text><text className="node-name" x="11" y="38">{compactName(node.name)}</text><text className="node-path" x="11" y="56">line {node.span.line}</text></g>;
          })}
        </g>
      </svg>}
    </div>
    {!list && <p className="map-readability" role={scale < .6 ? 'status' : undefined}>{scale < .6 ? 'Overview scale. Use Actual size or List view to read names.' : 'Drag to pan. Scroll to zoom.'}</p>}
    {!list && slice.edges.length === 0 && <p className="map-note">No detected relationships in this selection.</p>}
  </div>;
}
