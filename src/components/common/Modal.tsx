import { useEffect, useEffectEvent } from 'react';
import { clsx } from 'clsx';
import { X } from 'lucide-react';
import { Button } from './Button';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  size?: 'sm' | 'md' | 'lg' | 'xl';
  /** If true, modal can only be closed by buttons, not ESC or clicking outside */
  persistent?: boolean;
}

export function Modal({ isOpen, onClose, title, children, size = 'md', persistent = false }: ModalProps) {
  // React 19.2's useEffectEvent - creates stable callback without deps
  const handleClose = useEffectEvent(() => {
    if (isOpen && !persistent) {
      onClose();
    }
  });

  useEffect(() => {
    function handleEscape(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        handleClose();
      }
    }
    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, []); // No dependencies needed thanks to useEffectEvent

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50 dark:bg-black/70"
        onClick={persistent ? undefined : onClose}
      />

      {/* Modal */}
      <div
        className={clsx(
          'relative bg-bg-primary border border-border rounded-lg shadow-lg',
          'max-h-[85vh] flex flex-col',
          {
            'w-[400px]': size === 'sm',
            'w-[500px]': size === 'md',
            'w-[700px]': size === 'lg',
            'w-[900px]': size === 'xl',
          }
        )}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
          <h2 className="text-sm font-semibold text-text-primary">{title}</h2>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="w-4 h-4" />
          </Button>
        </div>

        {/* Content */}
        <div className="overflow-y-auto flex-1">{children}</div>
      </div>
    </div>
  );
}
