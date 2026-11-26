import React from 'react';
import { LinearCard } from './LinearCard';
import { LinearButton } from './LinearButton';

export interface DataTableColumn<T> {
    key: string;
    header: string;
    align?: 'left' | 'right' | 'center';
    className?: string;
    render: (row: T, index: number) => React.ReactNode;
}

export interface DataTableProps<T> {
    columns: DataTableColumn<T>[];
    rows: T[];
    rowKey: (row: T, index: number) => string;
    onRowClick?: (row: T) => void;
    loading?: boolean;
    error?: string | null;
    onRetry?: () => void;
    emptyTitle?: string;
    emptyDescription?: string;
    footerRow?: React.ReactNode;
    maxHeightClass?: string;
    caption?: string;
}

export function DataTable<T>({
    columns,
    rows,
    rowKey,
    onRowClick,
    loading = false,
    error = null,
    onRetry,
    emptyTitle = 'No data recorded',
    emptyDescription = 'Items will appear here once traffic is observed.',
    footerRow,
    maxHeightClass,
    caption,
}: DataTableProps<T>) {
    if (error) {
        return (
            <LinearCard level={1} className="p-5 border-[var(--red)] bg-[var(--chip-red)]">
                <div className="flex items-center justify-between gap-4">
                    <div className="space-y-1">
                        <h3 className="text-sm font-semibold text-[var(--red-text)]">
                            Could not load data
                        </h3>
                        <p className="text-xs text-[var(--text-secondary)]">{error}</p>
                    </div>
                    {onRetry && (
                        <LinearButton variant="secondary" size="sm" onClick={onRetry}>
                            Retry
                        </LinearButton>
                    )}
                </div>
            </LinearCard>
        );
    }

    if (loading && rows.length === 0) {
        return (
            <LinearCard level={1} className="p-5 space-y-3" role="status">
                {[1, 2, 3, 4].map((idx) => (
                    <div key={idx} className="h-8 rounded bg-[var(--bg-surface-2)] animate-pulse" />
                ))}
            </LinearCard>
        );
    }

    if (rows.length === 0) {
        return (
            <LinearCard level={1} className="p-8 text-center" role="status">
                <p className="text-sm font-semibold text-[var(--text)]">{emptyTitle}</p>
                <p className="mt-1 text-xs text-[var(--text-muted)]">{emptyDescription}</p>
            </LinearCard>
        );
    }

    const alignClass = (align?: 'left' | 'right' | 'center') =>
        align === 'right' ? 'text-right' : align === 'center' ? 'text-center' : 'text-left';

    return (
        <LinearCard level={1} className="overflow-hidden">
            <div
                className={`overflow-x-auto ${maxHeightClass ? `overflow-y-auto ${maxHeightClass}` : ''}`}
            >
                <table className="w-full border-collapse text-[13px]">
                    {caption && <caption className="sr-only">{caption}</caption>}
                    <thead className="sticky top-0 z-10 bg-[var(--bg-surface-2)] border-b border-[var(--border)]">
                        <tr>
                            {columns.map((col) => (
                                <th
                                    key={col.key}
                                    scope="col"
                                    className={`py-2.5 px-4 text-xs font-medium uppercase tracking-wider text-[var(--text-muted)] whitespace-nowrap ${alignClass(col.align)} ${col.className ?? ''}`}
                                >
                                    {col.header}
                                </th>
                            ))}
                        </tr>
                    </thead>
                    <tbody className="divide-y divide-[var(--border)]">
                        {rows.map((row, idx) => (
                            <tr
                                key={rowKey(row, idx)}
                                tabIndex={onRowClick ? 0 : undefined}
                                onClick={onRowClick ? () => onRowClick(row) : undefined}
                                onKeyDown={
                                    onRowClick
                                        ? (e) => {
                                              if (e.key === 'Enter' || e.key === ' ') {
                                                  e.preventDefault();
                                                  onRowClick(row);
                                              }
                                          }
                                        : undefined
                                }
                                className={`transition-colors duration-150 ease-out hover:bg-[var(--bg-hover)] ${
                                    onRowClick
                                        ? 'cursor-pointer focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-[var(--primary-text)]'
                                        : ''
                                }`}
                            >
                                {columns.map((col) => (
                                    <td
                                        key={col.key}
                                        className={`py-2.5 px-4 whitespace-nowrap ${alignClass(col.align)} ${col.className ?? ''}`}
                                    >
                                        {col.render(row, idx)}
                                    </td>
                                ))}
                            </tr>
                        ))}
                        {footerRow}
                    </tbody>
                </table>
            </div>
        </LinearCard>
    );
}
