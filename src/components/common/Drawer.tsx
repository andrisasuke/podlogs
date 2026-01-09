import { useEffect, useEffectEvent, Activity } from 'react';
import { clsx } from 'clsx';
import { X } from 'lucide-react';
import { Button } from './Button';
import { DRAWER_WIDTH } from '../../lib/constants';

interface DrawerProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
}

export function Drawer({ isOpen, onClose, title, children }: DrawerProps) {
  // React 19.2's useEffectEvent - creates stable callback without deps
  // The effect won't re-run when onClose changes, but will always use latest version
  const handleClose = useEffectEvent(() => {
    if (isOpen) {
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

  return (
    <>
      {/* Backdrop */}
      <div
        className={clsx(
          'fixed inset-0 bg-black/20 dark:bg-black/40 z-40 transition-opacity duration-300',
          isOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'
        )}
        onClick={onClose}
      />

      {/* Drawer */}
      <div
        className={clsx(
          'fixed top-0 right-0 h-full bg-bg-secondary border-l border-border z-50',
          'transition-transform duration-300 ease-out',
          isOpen ? 'translate-x-0' : 'translate-x-full'
        )}
        style={{ width: DRAWER_WIDTH }}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <h2 className="text-sm font-semibold text-text-primary truncate">{title}</h2>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="w-4 h-4" />
          </Button>
        </div>

        {/* Content - wrapped in Activity for React 19.2 */}
        {/* Activity preserves state when hidden and deprioritizes updates */}
        <div className="overflow-y-auto h-[calc(100%-56px)]">
          <Activity mode={isOpen ? 'visible' : 'hidden'}>
            {children}
          </Activity>
        </div>
      </div>
    </>
  );
}
