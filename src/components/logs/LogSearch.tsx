import { useState, useMemo, useRef, useCallback } from 'react';
import { clsx } from 'clsx';
import { Search, Download } from 'lucide-react';
import { Button } from '../common/Button';
import { Input } from '../common/Input';
import { Dropdown } from '../common/Dropdown';
import { LogLevelBadge } from '../common/Badge';
import { LogSkeleton } from '../common/Skeleton';
import { LogDetailModal } from './LogDetailModal';
import { useDeployments } from '../../hooks/useK8s';
import { useLogSearch } from '../../hooks/useLogs';
import { useClusterStore } from '../../stores/clusterStore';
import { formatShortTimestamp, highlightMatch } from '../../lib/formatters';
import { TIME_RANGES, getTimeRangeLabel, type TimeRange } from '../../lib/tauri';
import { LOG_LEVELS, type LogEntry } from '../../types/logs';

export function LogSearch() {
  const { deployment: selectedDeployment, setDeployment } = useClusterStore();
  const { data: deployments = [] } = useDeployments();

  const [keyword, setKeyword] = useState('');
  const [logLevel, setLogLevel] = useState<string>('');
  const [timeRange, setTimeRange] = useState<TimeRange>('1h');
  const [searchTriggered, setSearchTriggered] = useState(false);
  const [selectedLog, setSelectedLog] = useState<LogEntry | null>(null);

  // Resizable Pod column
  const [podColumnWidth, setPodColumnWidth] = useState(208); // w-52 = 13rem = 208px
  const isResizing = useRef(false);
  const startX = useRef(0);
  const startWidth = useRef(0);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    isResizing.current = true;
    startX.current = e.clientX;
    startWidth.current = podColumnWidth;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';

    const handleMouseMove = (e: MouseEvent) => {
      if (!isResizing.current) return;
      const diff = e.clientX - startX.current;
      const newWidth = Math.max(100, Math.min(500, startWidth.current + diff));
      setPodColumnWidth(newWidth);
    };

    const handleMouseUp = () => {
      isResizing.current = false;
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [podColumnWidth]);

  const {
    data: results = [],
    isLoading,
    isFetching,
  } = useLogSearch(
    selectedDeployment,
    { keyword, logLevel, timeRange },
    searchTriggered && !!selectedDeployment
  );

  const deploymentOptions = useMemo(
    () => deployments.map((d) => ({ value: d.name, label: d.name })),
    [deployments]
  );

  const logLevelOptions = [
    { value: '', label: 'Any' },
    ...LOG_LEVELS.map((l) => ({ value: l, label: l })),
  ];

  const timeRangeOptions = Object.keys(TIME_RANGES).map((key) => ({
    value: key,
    label: getTimeRangeLabel(key as TimeRange),
  }));

  const handleSearch = () => {
    if (selectedDeployment) {
      setSearchTriggered(true);
    }
  };

  const totalMatches = results.reduce((sum, r) => sum + r.total_matches, 0);
  const uniquePods = new Set(results.map((r) => r.pod_name)).size;

  const handleExport = () => {
    const lines = results.flatMap((r) =>
      r.entries.map(
        (e) => `${e.timestamp}\t${r.pod_name}\t${e.level || '-'}\t${e.message}`
      )
    );
    const content = lines.join('\n');
    const blob = new Blob([content], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${selectedDeployment}-log-search.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border">
        <h1 className="text-lg font-semibold text-text-primary mb-4">Log Search</h1>

        {/* Search Form */}
        <div className="grid grid-cols-4 gap-4">
          <div>
            <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
              Deployment
            </label>
            <Dropdown
              options={deploymentOptions}
              value={selectedDeployment || ''}
              onChange={setDeployment}
              placeholder="Select deployment..."
              searchable
              searchPlaceholder="Search deployments..."
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
              Keyword
            </label>
            <Input
              placeholder="Search keyword..."
              value={keyword}
              onChange={(e) => setKeyword(e.target.value)}
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
              Log Level
            </label>
            <Dropdown
              options={logLevelOptions}
              value={logLevel}
              onChange={setLogLevel}
            />
          </div>
          <div>
            <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
              Time Range
            </label>
            <Dropdown
              options={timeRangeOptions}
              value={timeRange}
              onChange={(v) => setTimeRange(v as TimeRange)}
            />
          </div>
        </div>

        <div className="flex justify-end mt-4">
          <Button
            variant="primary"
            onClick={handleSearch}
            disabled={!selectedDeployment || isFetching}
          >
            <Search className="w-4 h-4 mr-2" />
            Search
          </Button>
        </div>
      </div>

      {/* Results */}
      <div className="flex-1 overflow-hidden flex flex-col">
        {!searchTriggered ? (
          <div className="flex flex-col items-center justify-center h-full text-text-muted">
            <Search className="w-16 h-16 mb-4 opacity-30" />
            <p className="text-lg">Search across all pods in a deployment</p>
            <p className="text-sm mt-2">Select a deployment and click Search</p>
          </div>
        ) : isLoading ? (
          <LogSkeleton rows={15} />
        ) : results.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-text-muted">
            <Search className="w-12 h-12 mb-4 opacity-50" />
            <p className="text-lg">No results found</p>
            <p className="text-sm mt-1">Try adjusting your search filters</p>
          </div>
        ) : (
          <>
            {/* Results header - fixed */}
            <div className="flex items-center justify-between px-6 py-4">
              <h2 className="text-sm font-medium text-text-primary">Results</h2>
              <span className="text-sm text-text-muted">
                Found{' '}
                <span className="text-accent font-medium">{totalMatches}</span>{' '}
                matches across{' '}
                <span className="text-accent font-medium">{uniquePods}</span> pods
              </span>
            </div>

            {/* Results table */}
            <div className="flex-1 overflow-auto mx-6 mb-6 border border-border rounded-lg">
              <table className="w-full" style={{ tableLayout: 'fixed' }}>
                <thead className="sticky top-0 z-10">
                  <tr className="text-left text-xs font-medium text-text-muted uppercase tracking-wider bg-bg-secondary border-b border-border">
                    <th className="px-4 py-3 w-24">Timestamp</th>
                    <th className="px-4 py-3 relative" style={{ width: podColumnWidth }}>
                      Pod
                      {/* Resize handle */}
                      <div
                        onMouseDown={handleMouseDown}
                        className="absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-accent/50 active:bg-accent"
                        title="Drag to resize"
                      />
                    </th>
                    <th className="px-4 py-3 w-20">Level</th>
                    <th className="px-4 py-3">Message</th>
                  </tr>
                </thead>
                <tbody>
                  {results.flatMap((result) =>
                    result.entries.map((entry, idx) => (
                      <tr
                        key={`${result.pod_name}-${idx}`}
                        className={clsx(
                          'border-b border-border-subtle hover:bg-bg-tertiary/50 transition-colors cursor-pointer',
                          entry.level === 'ERROR' && 'bg-red-500/5',
                          entry.level === 'WARN' && 'bg-amber-500/5'
                        )}
                        onClick={() => setSelectedLog({ ...entry, pod_name: result.pod_name, container_name: result.container_name })}
                      >
                        <td className="px-4 py-2 font-mono text-sm text-text-muted w-24">
                          {formatShortTimestamp(entry.timestamp)}
                        </td>
                        <td className="px-4 py-2" style={{ width: podColumnWidth }}>
                          <span
                            className="font-mono text-sm text-cyan-500 dark:text-cyan-400 truncate block"
                            style={{ maxWidth: podColumnWidth - 32 }}
                          >
                            {result.pod_name}
                          </span>
                        </td>
                        <td className="px-4 py-2 w-20">
                          <LogLevelBadge level={entry.level} />
                        </td>
                        <td className="px-4 py-2">
                          <HighlightedText text={entry.message} keyword={keyword} />
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          </>
        )}
      </div>

      {/* Footer */}
      {results.length > 0 && (
        <div className="flex items-center justify-between px-6 py-2 border-t border-border">
          <div className="flex items-center gap-2 text-xs text-text-muted">
            {results.map((r) => (
              <span
                key={r.pod_name}
                className="inline-flex items-center gap-1 px-2 py-1 bg-bg-tertiary rounded"
              >
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                ...{r.pod_name.slice(-6)}
              </span>
            ))}
          </div>
          <Button variant="secondary" size="sm" onClick={handleExport}>
            <Download className="w-4 h-4 mr-1" />
            Export CSV
          </Button>
        </div>
      )}

      {/* Log Detail Modal */}
      <LogDetailModal
        entry={selectedLog}
        onClose={() => setSelectedLog(null)}
      />
    </div>
  );
}

function HighlightedText({ text, keyword }: { text: string; keyword: string }) {
  const match = highlightMatch(text, keyword);

  if (!match) {
    return (
      <span className="font-mono text-sm text-text-secondary truncate block">
        {text}
      </span>
    );
  }

  return (
    <span className="font-mono text-sm text-text-secondary truncate block">
      {match.before}
      <mark className="bg-amber-500/30 text-text-primary px-0.5 rounded">
        {match.match}
      </mark>
      {match.after}
    </span>
  );
}
