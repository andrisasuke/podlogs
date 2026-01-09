import { format, formatDistanceToNow, parseISO } from 'date-fns';

export function formatTimestamp(timestamp: string | null): string {
  if (!timestamp) return '-';
  try {
    const date = parseISO(timestamp);
    return format(date, 'yyyy-MM-dd HH:mm:ss');
  } catch {
    return timestamp;
  }
}

export function formatShortTimestamp(timestamp: string | null): string {
  if (!timestamp) return '-';
  try {
    const date = parseISO(timestamp);
    return format(date, 'HH:mm:ss');
  } catch {
    return timestamp;
  }
}

export function formatRelativeTime(timestamp: string | null): string {
  if (!timestamp) return '-';
  try {
    const date = parseISO(timestamp);
    return formatDistanceToNow(date, { addSuffix: true });
  } catch {
    return timestamp;
  }
}

export function formatAge(age: string): string {
  return age;
}

export function formatBytes(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let unitIndex = 0;
  let value = bytes;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex++;
  }

  return `${value.toFixed(1)} ${units[unitIndex]}`;
}

export function truncate(str: string, maxLength: number): string {
  if (str.length <= maxLength) return str;
  return str.slice(0, maxLength - 3) + '...';
}

export function highlightMatch(text: string, keyword: string): { before: string; match: string; after: string } | null {
  if (!keyword) return null;
  const lowerText = text.toLowerCase();
  const lowerKeyword = keyword.toLowerCase();
  const index = lowerText.indexOf(lowerKeyword);

  if (index === -1) return null;

  return {
    before: text.slice(0, index),
    match: text.slice(index, index + keyword.length),
    after: text.slice(index + keyword.length),
  };
}
