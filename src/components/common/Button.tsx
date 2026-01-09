import { ButtonHTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost';
  size?: 'sm' | 'md' | 'lg' | 'icon';
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant = 'secondary', size = 'md', children, ...props }, ref) => {
    return (
      <button
        ref={ref}
        className={clsx(
          'inline-flex items-center justify-center font-medium transition-colors duration-150',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2',
          'disabled:pointer-events-none disabled:opacity-50',
          {
            'bg-accent text-white hover:bg-accent-hover': variant === 'primary',
            'bg-transparent text-text-secondary border border-border hover:bg-bg-tertiary':
              variant === 'secondary',
            'bg-transparent text-text-secondary hover:bg-bg-tertiary': variant === 'ghost',
            'h-8 px-3 text-sm rounded-md': size === 'sm',
            'h-9 px-4 text-sm rounded-md': size === 'md',
            'h-10 px-6 text-base rounded-md': size === 'lg',
            'h-8 w-8 rounded-md': size === 'icon',
          },
          className
        )}
        {...props}
      >
        {children}
      </button>
    );
  }
);

Button.displayName = 'Button';
