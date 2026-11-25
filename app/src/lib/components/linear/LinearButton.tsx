import React from 'react';

export interface LinearButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
    size?: 'sm' | 'md' | 'lg';
}

export const LinearButton: React.FC<LinearButtonProps> = ({
    variant = 'secondary',
    size = 'md',
    type = 'button',
    className = '',
    children,
    ...props
}) => {
    const sizeStyles = {
        sm: 'h-7 px-2.5 text-xs',
        md: 'h-8 px-3 text-[13px]',
        lg: 'h-9 px-4 text-sm',
    }[size];

    const variantStyles = {
        primary:
            'bg-[var(--accent-blue)] hover:bg-[var(--accent-blue-hover)] active:bg-[var(--accent-blue-active)] text-[var(--on-accent)] font-medium border border-[var(--accent-blue)]',
        secondary:
            'bg-[var(--bg-surface-2)] hover:bg-[var(--bg-bar-idle)] text-[var(--text)] font-medium border border-[var(--border)] hover:border-[var(--border-strong)] shadow-[var(--shadow-card)] dark:shadow-none',
        ghost: 'bg-transparent hover:bg-[var(--bg-surface-2)] text-[var(--text-muted)] hover:text-[var(--text)] font-medium border border-transparent',
        danger: 'bg-[var(--tint-red)] hover:border-[var(--accent-red)] text-[var(--tint-red-fg)] font-medium border border-[var(--border-strong)]',
    }[variant];

    return (
        <button
            type={type}
            className={`inline-flex items-center justify-center gap-1.5 rounded-md font-sans transition-colors duration-150 ease-out focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--accent-blue)] disabled:opacity-50 disabled:pointer-events-none cursor-pointer ${sizeStyles} ${variantStyles} ${className}`}
            {...props}
        >
            {children}
        </button>
    );
};
