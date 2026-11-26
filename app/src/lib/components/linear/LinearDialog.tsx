import React, { useEffect, useId, useRef } from 'react';
import { X } from 'lucide-react';

export interface LinearDialogProps {
    open: boolean;
    onClose: () => void;
    title: string;
    subtitle?: string;
    maxWidthClass?: string;
    footer?: React.ReactNode;
    children: React.ReactNode;
}

export const LinearDialog: React.FC<LinearDialogProps> = ({
    open,
    onClose,
    title,
    subtitle,
    maxWidthClass = 'max-w-lg',
    footer,
    children,
}) => {
    const titleId = useId();
    const panelRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (!open) return;
        const prev = document.activeElement instanceof HTMLElement ? document.activeElement : null;
        panelRef.current?.focus();

        const onKeyDown = (e: KeyboardEvent) => {
            if (e.key === 'Escape') {
                e.preventDefault();
                onClose();
            }
        };
        window.addEventListener('keydown', onKeyDown);
        return () => {
            window.removeEventListener('keydown', onKeyDown);
            if (prev && prev.isConnected) prev.focus();
        };
    }, [open, onClose]);

    if (!open) return null;

    return (
        <div
            className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-[var(--scrim)]"
            role="presentation"
            onClick={(e) => {
                if (e.target === e.currentTarget) onClose();
            }}
        >
            <div
                ref={panelRef}
                role="dialog"
                aria-modal="true"
                aria-labelledby={titleId}
                tabIndex={-1}
                className={`w-full ${maxWidthClass} max-h-[85vh] flex flex-col rounded-lg border border-[var(--border-strong)] bg-[var(--bg-surface)] shadow-card overflow-hidden focus:outline-none`}
            >
                <header className="px-5 py-4 border-b border-[var(--border)] flex items-start justify-between gap-4 shrink-0">
                    <div>
                        {subtitle && (
                            <p className="text-xs font-medium text-[var(--text-muted)] mb-1">
                                {subtitle}
                            </p>
                        )}
                        <h2 id={titleId} className="text-base font-semibold text-[var(--text)]">
                            {title}
                        </h2>
                    </div>
                    <button
                        type="button"
                        onClick={onClose}
                        aria-label="Close dialog"
                        className="size-7 rounded-md inline-flex items-center justify-center text-[var(--text-muted)] hover:text-[var(--text)] hover:bg-[var(--bg-hover)] transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-[var(--primary-text)] cursor-pointer"
                    >
                        <X className="size-4" aria-hidden="true" />
                    </button>
                </header>

                <div className="p-5 overflow-y-auto flex-1">{children}</div>

                {footer && (
                    <footer className="px-5 py-3.5 border-t border-[var(--border)] bg-[var(--bg-surface-2)] flex items-center justify-end gap-2 shrink-0">
                        {footer}
                    </footer>
                )}
            </div>
        </div>
    );
};
