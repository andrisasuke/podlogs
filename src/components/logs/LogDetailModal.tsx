import { useState } from 'react';
import { clsx } from 'clsx';
import { Copy, Check } from 'lucide-react';
import { Button } from '../common/Button';
import { Modal } from '../common/Modal';
import { LogLevelBadge } from '../common/Badge';
import { formatTimestamp } from '../../lib/formatters';
import type { LogEntry } from '../../types/logs';

interface LogDetailModalProps {
  entry: LogEntry | null;
  onClose: () => void;
}

export function LogDetailModal({ entry, onClose }: LogDetailModalProps) {
  const [copiedMessage, setCopiedMessage] = useState(false);
  const [copiedRaw, setCopiedRaw] = useState(false);
  const [copiedJson, setCopiedJson] = useState(false);

  const handleCopyMessage = async () => {
    if (!entry) return;
    await navigator.clipboard.writeText(entry.message);
    setCopiedMessage(true);
    setTimeout(() => setCopiedMessage(false), 2000);
  };

  const handleCopyRaw = async () => {
    if (!entry) return;
    await navigator.clipboard.writeText(entry.raw);
    setCopiedRaw(true);
    setTimeout(() => setCopiedRaw(false), 2000);
  };

  const getFormattedJson = () => {
    if (!entry) return '';
    try {
      const jsonStart = entry.raw.indexOf('{');
      if (jsonStart >= 0) {
        const jsonStr = entry.raw.slice(jsonStart);
        return JSON.stringify(JSON.parse(jsonStr), null, 2);
      }
      return entry.raw;
    } catch {
      return entry.raw;
    }
  };

  const handleCopyJson = async () => {
    if (!entry) return;
    await navigator.clipboard.writeText(getFormattedJson());
    setCopiedJson(true);
    setTimeout(() => setCopiedJson(false), 2000);
  };

  if (!entry) return null;

  return (
    <Modal isOpen={!!entry} onClose={onClose} title="Log Details" size="lg">
      <div className="p-4 space-y-4">
        {/* Metadata */}
        <div className="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span className="text-text-muted">Timestamp</span>
            <p className="font-mono text-text-primary mt-1">
              {formatTimestamp(entry.timestamp)}
            </p>
          </div>
          <div>
            <span className="text-text-muted">Level</span>
            <div className="mt-1">
              <LogLevelBadge level={entry.level} />
            </div>
          </div>
          <div>
            <span className="text-text-muted">Pod</span>
            <p className="font-mono text-text-primary mt-1">{entry.pod_name}</p>
          </div>
          <div>
            <span className="text-text-muted">Container</span>
            <p className="font-mono text-text-primary mt-1">{entry.container_name}</p>
          </div>
        </div>

        {/* Message */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-text-muted">Message</span>
            <Button variant="ghost" size="sm" onClick={handleCopyMessage}>
              {copiedMessage ? (
                <>
                  <Check className="w-4 h-4 mr-1 text-emerald-500" />
                  Copied
                </>
              ) : (
                <>
                  <Copy className="w-4 h-4 mr-1" />
                  Copy
                </>
              )}
            </Button>
          </div>
          <div
            className={clsx(
              'p-3 rounded-md bg-bg-tertiary font-mono text-sm whitespace-pre-wrap break-all max-h-[200px] overflow-auto',
              entry.level === 'ERROR' && 'text-red-500 dark:text-red-400',
              entry.level === 'WARN' && 'text-amber-600 dark:text-amber-400',
              !entry.level && 'text-text-primary'
            )}
          >
            {entry.message}
          </div>
        </div>

        {/* Raw Log */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-text-muted">Raw Log</span>
            <Button variant="ghost" size="sm" onClick={handleCopyRaw}>
              {copiedRaw ? (
                <>
                  <Check className="w-4 h-4 mr-1 text-emerald-500" />
                  Copied
                </>
              ) : (
                <>
                  <Copy className="w-4 h-4 mr-1" />
                  Copy
                </>
              )}
            </Button>
          </div>
          <div className="p-3 rounded-md bg-bg-secondary border border-border font-mono text-xs text-text-secondary whitespace-pre-wrap break-all max-h-[150px] overflow-auto">
            {entry.raw}
          </div>
        </div>

        {/* JSON View (if applicable) */}
        {entry.is_json && (
          <div>
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm text-text-muted">JSON (Formatted)</span>
              <Button variant="ghost" size="sm" onClick={handleCopyJson}>
                {copiedJson ? (
                  <>
                    <Check className="w-4 h-4 mr-1 text-emerald-500" />
                    Copied
                  </>
                ) : (
                  <>
                    <Copy className="w-4 h-4 mr-1" />
                    Copy
                  </>
                )}
              </Button>
            </div>
            <div className="p-3 rounded-md bg-bg-secondary border border-border font-mono text-xs text-text-secondary whitespace-pre-wrap break-all max-h-[200px] overflow-auto">
              {getFormattedJson()}
            </div>
          </div>
        )}
      </div>
    </Modal>
  );
}
