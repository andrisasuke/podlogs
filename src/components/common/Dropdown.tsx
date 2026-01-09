import { useState, useRef, useEffect, useMemo } from 'react';
import { clsx } from 'clsx';
import { ChevronDown, Check, Search } from 'lucide-react';

interface DropdownOption<T = string> {
  value: T;
  label: string;
}

interface DropdownProps<T = string> {
  options: DropdownOption<T>[];
  value: T;
  onChange: (value: T) => void;
  placeholder?: string;
  className?: string;
  disabled?: boolean;
  searchable?: boolean;
  searchPlaceholder?: string;
}

export function Dropdown<T extends string>({
  options,
  value,
  onChange,
  placeholder = 'Select...',
  className,
  disabled = false,
  searchable = false,
  searchPlaceholder = 'Search...',
}: DropdownProps<T>) {
  const [isOpen, setIsOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const dropdownRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setIsOpen(false);
        setSearchQuery('');
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Focus search input when dropdown opens
  useEffect(() => {
    if (isOpen && searchable && searchInputRef.current) {
      searchInputRef.current.focus();
    }
  }, [isOpen, searchable]);

  const selectedOption = options.find((opt) => opt.value === value);

  const filteredOptions = useMemo(() => {
    if (!searchable || !searchQuery) return options;
    const query = searchQuery.toLowerCase();
    return options.filter((opt) => opt.label.toLowerCase().includes(query));
  }, [options, searchQuery, searchable]);

  return (
    <div ref={dropdownRef} className={clsx('relative', className)}>
      <button
        type="button"
        onClick={() => !disabled && setIsOpen(!isOpen)}
        disabled={disabled}
        className={clsx(
          'w-full h-9 px-3 flex items-center justify-between gap-2 rounded-md text-sm',
          'bg-bg-tertiary border border-border',
          'text-text-primary',
          'hover:bg-bg-secondary transition-colors duration-150',
          'focus:outline-none focus:ring-2 focus:ring-accent',
          'disabled:opacity-50 disabled:cursor-not-allowed'
        )}
      >
        <span className={clsx(!selectedOption && 'text-text-muted')}>
          {selectedOption?.label || placeholder}
        </span>
        <ChevronDown
          className={clsx(
            'w-4 h-4 text-text-muted transition-transform duration-200',
            isOpen && 'rotate-180'
          )}
        />
      </button>

      {isOpen && (
        <div
          className={clsx(
            'absolute z-50 w-full mt-1 rounded-md shadow-md',
            'bg-bg-secondary border border-border',
            'max-h-[300px] overflow-hidden flex flex-col'
          )}
        >
          {/* Search input */}
          {searchable && (
            <div className="p-2 border-b border-border">
              <div className="relative">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-4 h-4 text-text-muted" />
                <input
                  ref={searchInputRef}
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder={searchPlaceholder}
                  autoCapitalize="off"
                  autoCorrect="off"
                  spellCheck={false}
                  className={clsx(
                    'w-full h-8 pl-8 pr-3 rounded-md text-sm',
                    'bg-bg-tertiary border border-border',
                    'text-text-primary placeholder:text-text-muted',
                    'focus:outline-none focus:ring-2 focus:ring-accent'
                  )}
                />
              </div>
            </div>
          )}

          {/* Options list */}
          <div className="overflow-auto py-1">
            {filteredOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => {
                  onChange(option.value);
                  setIsOpen(false);
                  setSearchQuery('');
                }}
                className={clsx(
                  'w-full px-3 py-2 flex items-center gap-2 text-sm text-left',
                  'hover:bg-bg-tertiary transition-colors duration-150',
                  option.value === value && 'text-accent'
                )}
              >
                {option.value === value ? (
                  <Check className="w-4 h-4" />
                ) : (
                  <span className="w-4" />
                )}
                {option.label}
              </button>
            ))}
            {filteredOptions.length === 0 && (
              <div className="px-3 py-2 text-sm text-text-muted">No results found</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
