import { clsx } from 'clsx';
import type { LogLevel } from '../../types/logs';
import type { PodStatus } from '../../types/kubernetes';

interface BadgeProps {
  children: React.ReactNode;
  variant?: 'default' | 'success' | 'warning' | 'error' | 'info' | 'debug';
  className?: string;
}

export function Badge({ children, variant = 'default', className }: BadgeProps) {
  return (
    <span
      className={clsx(
        'inline-flex items-center px-2 py-0.5 rounded text-xs font-medium',
        {
          'bg-bg-tertiary text-text-secondary': variant === 'default',
          'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400': variant === 'success',
          'bg-amber-500/10 text-amber-600 dark:text-amber-400': variant === 'warning',
          'bg-red-500/10 text-red-600 dark:text-red-400': variant === 'error',
          'bg-blue-500/10 text-blue-600 dark:text-blue-400': variant === 'info',
          'bg-purple-500/10 text-purple-600 dark:text-purple-400': variant === 'debug',
        },
        className
      )}
    >
      {children}
    </span>
  );
}

interface StatusBadgeProps {
  status: string;
  ready?: string; // e.g., "0/1", "1/1"
  className?: string;
}

export function StatusBadge({ status, ready, className }: StatusBadgeProps) {
  const variant = getStatusVariant(status as PodStatus, ready);
  return (
    <span
      className={clsx(
        'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium',
        {
          'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400': variant === 'success',
          'bg-amber-500/10 text-amber-600 dark:text-amber-400': variant === 'warning',
          'bg-red-500/10 text-red-600 dark:text-red-400': variant === 'error',
          'bg-purple-500/10 text-purple-600 dark:text-purple-400': variant === 'terminating',
          'bg-gray-500/10 text-gray-500 dark:text-gray-400': variant === 'default',
        },
        className
      )}
    >
      <span
        className={clsx('w-1.5 h-1.5 rounded-full', {
          'bg-emerald-500': variant === 'success',
          'bg-amber-500': variant === 'warning',
          'bg-red-500': variant === 'error',
          'bg-purple-500': variant === 'terminating',
          'bg-gray-500': variant === 'default',
        })}
      />
      {status}
    </span>
  );
}

function getStatusVariant(status: PodStatus, ready?: string): 'success' | 'warning' | 'error' | 'terminating' | 'default' {
  // Check if pod is terminating
  if (status === 'Terminating') {
    return 'terminating';
  }

  // Check if pod is running but not all containers are ready (e.g., "0/1", "1/2")
  if (status === 'Running' && ready) {
    const [readyCount, totalCount] = ready.split('/').map(Number);
    if (!isNaN(readyCount) && !isNaN(totalCount) && readyCount < totalCount) {
      return 'warning'; // Show amber for not fully ready pods
    }
  }

  switch (status) {
    case 'Running':
    case 'Succeeded':
      return 'success';
    case 'Pending':
    case 'ContainerCreating':
      return 'warning';
    case 'Failed':
    case 'CrashLoopBackOff':
    case 'ImagePullBackOff':
    case 'ErrImagePull':
    case 'Evicted':
    case 'UnexpectedAdmissionError':
    case 'ContainerStatusUnknown':
      return 'error';
    default:
      return 'default';
  }
}

interface LogLevelBadgeProps {
  level: LogLevel | null;
  className?: string;
}

export function LogLevelBadge({ level, className }: LogLevelBadgeProps) {
  if (!level) return null;

  const variant = {
    ERROR: 'error',
    WARN: 'warning',
    INFO: 'info',
    DEBUG: 'debug',
  }[level] as 'error' | 'warning' | 'info' | 'debug';

  return (
    <span
      className={clsx(
        'inline-flex items-center justify-center min-w-[52px] px-2 py-0.5 rounded text-xs font-medium text-white',
        {
          'bg-red-500 dark:bg-red-600': variant === 'error',
          'bg-amber-500': variant === 'warning',
          'bg-blue-500': variant === 'info',
          'bg-purple-500': variant === 'debug',
        },
        className
      )}
    >
      {level}
    </span>
  );
}

interface LabelBadgeProps {
  label: string;
  className?: string;
}

export function LabelBadge({ label, className }: LabelBadgeProps) {
  return (
    <span
      className={clsx(
        'inline-flex items-center px-2 py-1 rounded text-xs font-mono',
        'bg-bg-tertiary text-text-secondary',
        className
      )}
    >
      {label}
    </span>
  );
}
