import { InputHTMLAttributes, forwardRef } from 'react';
import { clsx } from 'clsx';
import { Search } from 'lucide-react';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  icon?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, icon = false, ...props }, ref) => {
    if (icon) {
      return (
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
          <input
            ref={ref}
            autoCapitalize="off"
            autoCorrect="off"
            spellCheck={false}
            className={clsx(
              'h-9 w-full pl-9 pr-3 rounded-md text-sm',
              'bg-bg-tertiary border border-border',
              'text-text-primary placeholder:text-text-muted',
              'focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent',
              'transition-colors duration-150',
              className
            )}
            {...props}
          />
        </div>
      );
    }

    return (
      <input
        ref={ref}
        className={clsx(
          'h-9 w-full px-3 rounded-md text-sm',
          'bg-bg-tertiary border border-border',
          'text-text-primary placeholder:text-text-muted',
          'focus:outline-none focus:ring-2 focus:ring-accent focus:border-transparent',
          'transition-colors duration-150',
          className
        )}
        {...props}
      />
    );
  }
);

Input.displayName = 'Input';
