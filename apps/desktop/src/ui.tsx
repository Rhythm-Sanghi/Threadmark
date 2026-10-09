import type { ReactNode } from 'react';

export function SectionHeading({ children, index, meta }: { children: ReactNode; index?: string; meta?: ReactNode }) {
  return <div className="section-heading"><h2>{index && <span className="section-index" aria-hidden="true">{index} / </span>}{children}</h2>{meta && <span className="section-meta">{meta}</span>}</div>;
}

export function PageHeading({ title, description }: { title: string; description: string }) {
  return <div className="page-heading"><h1>{title}</h1><p>{description}</p></div>;
}

export function EmptyState({ title, children }: { title: string; children?: ReactNode }) {
  return <div className="empty-state"><h3>{title}</h3>{children && <p>{children}</p>}</div>;
}
