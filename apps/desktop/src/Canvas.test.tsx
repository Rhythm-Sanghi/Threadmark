import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, it, vi } from 'vitest';
import { Canvas } from './Canvas';
import type { Slice } from './api';

const slice: Slice = {
  schema_version: 1, focus: '0', truncated: true,
  nodes: Array.from({ length: 24 }, (_, index) => ({
    id: String(index), name: `calculateForecastForRegionalOffice${index}`,
    qualified_name: `forecast.calculateForecastForRegionalOffice${index}`,
    kind: 'function', language: 'typescript', path: 'src/forecast.ts',
    span: { start_byte: 0, end_byte: 20, line: index + 1, column: 1, end_line: index + 1, end_column: 21 },
    exported: true, test: false,
  })),
  edges: Array.from({ length: 23 }, (_, index) => ({
    id: `edge${index}`, from: '0', to: String(index + 1), kind: 'CALLS',
    confidence: 'RESOLVED', path: 'src/forecast.ts',
    span: { start_byte: 0, end_byte: 20, line: 1, column: 1, end_line: 1, end_column: 21 },
    evidence: `calculateForecastForRegionalOffice${index + 1}()`, adapter: 'typescript',
  })),
};
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

it('offers readable names and working keyboard actions for a full neighborhood', async () => {
  const onNode = vi.fn(), onContext = vi.fn();
  render(<Canvas slice={slice} onNode={onNode} onEdge={vi.fn()} onContext={onContext} />);
  await userEvent.click(screen.getByRole('button', { name: 'Fit selection' }));
  expect(screen.getByRole('status').textContent).toContain('Overview scale');
  await userEvent.click(screen.getByRole('button', { name: 'List view' }));
  expect(screen.getAllByRole('row')).toHaveLength(25);
  const target = screen.getByRole('button', { name: slice.nodes[23].qualified_name });
  target.focus();
  await userEvent.keyboard('{Shift>}{F10}{/Shift}');
  expect(onContext).toHaveBeenCalledWith(slice.nodes[23], expect.any(Number), expect.any(Number));
  await userEvent.keyboard('{Enter}');
  expect(onNode).toHaveBeenCalledWith(slice.nodes[23]);
  await userEvent.click(screen.getByRole('button', { name: 'Map view' }));
  expect(screen.getByRole('status')).toBeTruthy();
  await userEvent.click(screen.getByRole('button', { name: 'Actual size' }));
  expect(screen.queryByRole('status')).toBeNull();
  expect(screen.getByText('100% · 24 nodes · limit reached')).toBeTruthy();
});

it('retains full node titles and keyboard evidence at overview scale', async () => {
  const onEdge = vi.fn();
  const { container } = render(<Canvas slice={slice} onNode={vi.fn()} onEdge={onEdge} />);
  await userEvent.click(screen.getByRole('button', { name: 'Fit selection' }));
  expect(container.querySelector('.node-name')?.getAttribute('textLength')).toBeNull();
  expect(container.querySelector('.map-node title')?.textContent).toContain(slice.nodes[0].qualified_name);
  const edge = screen.getByRole('button', { name: `CALLS: ${slice.nodes[0].name} to ${slice.nodes[1].name}` });
  edge.focus();
  await userEvent.keyboard('{Enter}');
  expect(onEdge).toHaveBeenCalledWith(slice.edges[0]);
});

it('fits a resized viewport and restores physical size without losing navigation', async () => {
  const disconnect = vi.fn();
  vi.stubGlobal('ResizeObserver', class {
    constructor(private callback: ResizeObserverCallback) {}
    observe() { this.callback([{ contentRect: { width: 420, height: 300 } } as ResizeObserverEntry], this as unknown as ResizeObserver); }
    disconnect = disconnect;
  });
  const onNode = vi.fn();
  const { unmount } = render(<Canvas slice={slice} onNode={onNode} onEdge={vi.fn()} />);
  expect(screen.getByLabelText('Relationship map').getAttribute('viewBox')).toBe('0 0 420 300');
  await userEvent.click(screen.getByRole('button', { name: 'Actual size' }));
  expect(screen.getByText('100% · 24 nodes · limit reached')).toBeTruthy();
  fireEvent.keyDown(screen.getByRole('button', { name: `Focus ${slice.nodes[0].name}` }), { key: 'Enter' });
  expect(onNode).toHaveBeenCalledWith(slice.nodes[0]);
  unmount();
  expect(disconnect).toHaveBeenCalledOnce();
});
