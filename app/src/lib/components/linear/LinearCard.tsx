import React, { forwardRef } from 'react';

export interface LinearCardProps extends React.HTMLAttributes<HTMLDivElement> {
    level?: 1 | 2;
}

export const LinearCard = forwardRef<HTMLDivElement, LinearCardProps>(
    ({ level = 1, className = '', children, ...props }, ref) => {
        const surfaceClass = level === 2 ? 'bg-[var(--bg-surface-2)]' : 'bg-[var(--bg-surface)]';

        return (
            <div
                ref={ref}
                className={`rounded-lg border border-[var(--border)] shadow-[var(--shadow-card)] dark:shadow-none transition-colors duration-150 ease-out text-[var(--text)] ${surfaceClass} ${className}`}
                {...props}
            >
                {children}
            </div>
        );
    },
);

LinearCard.displayName = 'LinearCard';
