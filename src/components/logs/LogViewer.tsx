import { useRef, useEffect, useEffectEvent, useState, useMemo } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { clsx } from 'clsx';
import { ArrowLeft, Download } from 'lucide-react';
import { Button } from '../common/Button';
import { Input } from '../common/Input';
import { Dropdown } from '../common/Dropdown';
import { LogLevelBadge } from '../common/Badge';
import { LogSkeleton } from '../common/Skeleton';
import { LogDetailModal } from './LogDetailModal';
import { usePodLogs } from '../../hooks/useLogs';
import { usePodDetails } from '../../hooks/useK8s';
import { useUIStore } from '../../stores/uiStore';
import { formatTimestamp } from '../../lib/formatters';
import { TIME_RANGES, getTimeRangeLabel, type TimeRange } from '../../lib/tauri';
import type { LogEntry } from '../../types/logs';

interface LogViewerProps {
  podName: string | null;
}

export function LogViewer({ podName }: LogViewerProps) {
  const { closeLogViewer } = useUIStore();
  const { data: podDetails } = usePodDetails(podName);

  const [container, setContainer] = useState<string>('');
  const [timeRange, setTimeRange] = useState<TimeRange>('30m');
  const [filter, setFilter] = useState('');
  const [viewMode, setViewMode] = useState<'json' | 'raw'>('json');
  const [autoScroll, setAutoScroll] = useState(true);
  const [selectedLog, setSelectedLog] = useState<LogEntry | null>(null);

  const parentRef = useRef<HTMLDivElement>(null);

  // Set initial container when pod details load
  useEffect(() => {
    if (podDetails?.containers?.[0]?.name && !container) {
      setContainer(podDetails.containers[0].name);
    }
  }, [podDetails, container]);

  const {
    data: logs = [],
    isLoading,
    isFetching,
  } = usePodLogs(podName, {
    container,
    timeRange,
    enabled: !!podName && !!container,
  });

  // Filter logs
  const filteredLogs = useMemo(() => {
    if (!filter) return logs;
    const lowerFilter = filter.toLowerCase();
    return logs.filter(
      (log) =>
        log.message.toLowerCase().includes(lowerFilter) ||
        log.raw.toLowerCase().includes(lowerFilter)
    );
  }, [logs, filter]);

  // Virtualization
  const virtualizer = useVirtualizer({
    count: filteredLogs.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => (viewMode === 'json' ? 36 : 24),
    overscan: 20,
  });

  // React 19.2's useEffectEvent - creates stable scroll function
  // The effect triggers on log count change, but scrolling logic doesn't need to be in deps
  const scrollToBottom = useEffectEvent(() => {
    if (autoScroll && parentRef.current) {
      parentRef.current.scrollTop = parentRef.current.scrollHeight;
    }
  });

  // Auto-scroll when logs change
  useEffect(() => {
    if (filteredLogs.length > 0) {
      scrollToBottom();
    }
  }, [filteredLogs.length]); // Only trigger on count change, scrollToBottom is stable

  const containerOptions = useMemo(
    () =>
      podDetails?.containers?.map((c) => ({
        value: c.name,
        label: c.name,
      })) || [],
    [podDetails]
  );

  const timeRangeOptions = Object.keys(TIME_RANGES).map((key) => ({
    value: key,
    label: getTimeRangeLabel(key as TimeRange),
  }));

  const handleExport = () => {
    const content = filteredLogs.map((l) => l.raw).join('\n');
    const blob = new Blob([content], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${podName}-${container}-logs.txt`;
    a.click();
    URL.revokeObjectURL(url);
  };

  if (!podName) {
    return (
      <div className="flex items-center justify-center h-full text-text-muted">
        Select a pod to view logs
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border">
        <div className="flex items-center gap-4 mb-4">
          <Button variant="ghost" size="sm" onClick={closeLogViewer}>
            <ArrowLeft className="w-4 h-4 mr-1" />
            Back
          </Button>
          <div>
            <h1 className="text-lg font-semibold text-text-primary">Logs</h1>
            <p className="text-sm text-text-muted font-mono">{podName}</p>
          </div>
        </div>

        {/* Controls */}
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <span className="text-sm text-text-muted">Container:</span>
            <Dropdown
              options={containerOptions}
              value={container}
              onChange={setContainer}
              className="w-40"
            />
          </div>
          <div className="flex items-center gap-2">
            <span className="text-sm text-text-muted">Time:</span>
            <Dropdown
              options={timeRangeOptions}
              value={timeRange}
              onChange={(v) => setTimeRange(v as TimeRange)}
              className="w-44"
            />
          </div>
          <div className="flex-1">
            <Input
              icon
              placeholder="Filter logs..."
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
            />
          </div>
          <div className="flex items-center rounded-md border border-border overflow-hidden">
            <button
              onClick={() => setViewMode('json')}
              className={clsx(
                'px-3 py-1.5 text-sm font-medium transition-colors',
                viewMode === 'json'
                  ? 'bg-accent text-white'
                  : 'bg-transparent text-text-secondary hover:bg-bg-tertiary'
              )}
            >
              JSON
            </button>
            <button
              onClick={() => setViewMode('raw')}
              className={clsx(
                'px-3 py-1.5 text-sm font-medium transition-colors',
                viewMode === 'raw'
                  ? 'bg-accent text-white'
                  : 'bg-transparent text-text-secondary hover:bg-bg-tertiary'
              )}
            >
              Raw
            </button>
          </div>
        </div>
      </div>

      {/* Logs */}
      {isLoading ? (
        <LogSkeleton rows={15} />
      ) : (
        <>
          {/* Table Header for JSON mode */}
          {viewMode === 'json' && (
            <div className="flex items-center px-6 py-2 border-b border-border text-xs font-medium text-text-muted uppercase tracking-wider">
              <span className="w-44">Timestamp</span>
              <span className="w-16">Level</span>
              <span className="flex-1">Message</span>
            </div>
          )}

          <div
            ref={parentRef}
            className="flex-1 overflow-auto font-mono text-sm"
            onScroll={(e) => {
              const { scrollTop, scrollHeight, clientHeight } = e.currentTarget;
              const isAtBottom = scrollHeight - scrollTop - clientHeight < 100;
              setAutoScroll(isAtBottom);
            }}
          >
            <div
              style={{
                height: `${virtualizer.getTotalSize()}px`,
                position: 'relative',
              }}
            >
              {virtualizer.getVirtualItems().map((virtualRow) => (
                <LogRow
                  key={virtualRow.key}
                  entry={filteredLogs[virtualRow.index]}
                  viewMode={viewMode}
                  onClick={() => setSelectedLog(filteredLogs[virtualRow.index])}
                  style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    width: '100%',
                    height: `${virtualRow.size}px`,
                    transform: `translateY(${virtualRow.start}px)`,
                  }}
                />
              ))}
            </div>
          </div>
        </>
      )}

      {/* Footer */}
      <div className="flex items-center justify-between px-6 py-2 border-t border-border text-sm text-text-muted">
        <div className="flex items-center gap-3">
          {isFetching ? (
            <span className="flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
              Syncing...
            </span>
          ) : (
            <span className="flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-emerald-500" />
              Live
            </span>
          )}
          <span>{filteredLogs.length.toLocaleString()} lines</span>
          <span>Auto-scroll: {autoScroll ? 'ON' : 'OFF'}</span>
        </div>
        <Button variant="secondary" size="sm" onClick={handleExport}>
          <Download className="w-4 h-4 mr-1" />
          Export
        </Button>
      </div>

      {/* Log Detail Modal */}
      <LogDetailModal
        entry={selectedLog}
        onClose={() => setSelectedLog(null)}
      />
    </div>
  );
}

interface LogRowProps {
  entry: LogEntry;
  viewMode: 'json' | 'raw';
  style: React.CSSProperties;
  onClick: () => void;
}

function LogRow({ entry, viewMode, style, onClick }: LogRowProps) {
  if (viewMode === 'raw') {
    return (
      <div
        style={style}
        onClick={onClick}
        className={clsx(
          'px-6 py-1 text-text-secondary hover:bg-bg-tertiary/50 whitespace-pre truncate cursor-pointer',
          entry.level === 'ERROR' && 'text-red-500 dark:text-red-400',
          entry.level === 'WARN' && 'text-amber-600 dark:text-amber-400'
        )}
      >
        {entry.raw}
      </div>
    );
  }

  return (
    <div
      style={style}
      onClick={onClick}
      className={clsx(
        'flex items-center px-6 py-1.5 hover:bg-bg-tertiary/50 cursor-pointer',
        entry.level === 'ERROR' && 'bg-red-500/5',
        entry.level === 'WARN' && 'bg-amber-500/5'
      )}
    >
      <span className="w-44 text-text-muted shrink-0">
        {formatTimestamp(entry.timestamp)}
      </span>
      <span className="w-16 shrink-0">
        <LogLevelBadge level={entry.level} />
      </span>
      <span
        className={clsx(
          'flex-1 truncate',
          entry.level === 'ERROR' && 'text-red-500 dark:text-red-400',
          entry.level === 'WARN' && 'text-amber-600 dark:text-amber-400',
          !entry.level && 'text-text-secondary'
        )}
      >
        {entry.message}
      </span>
    </div>
  );
}

