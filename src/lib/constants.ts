export const REFETCH_INTERVAL = 5000; // 5 seconds
export const LOG_REFETCH_INTERVAL = 5000;

export const SIDEBAR_WIDTH = 260;
export const SIDEBAR_COLLAPSED_WIDTH = 60;
export const DRAWER_WIDTH = 480;
export const HEADER_HEIGHT = 56;
export const STATUS_BAR_HEIGHT = 28;

export const MIN_WINDOW_WIDTH = 1000;
export const MIN_WINDOW_HEIGHT = 700;
export const DEFAULT_WINDOW_WIDTH = 1400;
export const DEFAULT_WINDOW_HEIGHT = 900;

export const POD_STATUS_COLORS = {
  Running: 'status-success',
  Pending: 'status-warning',
  Succeeded: 'status-success',
  Failed: 'status-error',
  Unknown: 'text-muted',
  CrashLoopBackOff: 'status-error',
  ImagePullBackOff: 'status-error',
} as const;

export const LOG_LEVEL_COLORS = {
  ERROR: {
    bg: 'bg-red-500/10',
    text: 'text-red-500 dark:text-red-400',
    badge: 'bg-red-500 dark:bg-red-600',
  },
  WARN: {
    bg: 'bg-amber-500/10',
    text: 'text-amber-600 dark:text-amber-400',
    badge: 'bg-amber-500 dark:bg-amber-500',
  },
  INFO: {
    bg: 'bg-transparent',
    text: 'text-blue-500 dark:text-blue-400',
    badge: 'bg-blue-500 dark:bg-blue-500',
  },
  DEBUG: {
    bg: 'bg-purple-500/5',
    text: 'text-purple-500 dark:text-purple-400',
    badge: 'bg-purple-500 dark:bg-purple-500',
  },
} as const;
