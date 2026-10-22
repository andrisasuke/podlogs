import { useState, useMemo, useRef, useCallback, useEffect } from 'react';
import { clsx } from 'clsx';
import { Search, Download, Settings } from 'lucide-react';
import { Button } from '../common/Button';
import { Input } from '../common/Input';
import { Dropdown } from '../common/Dropdown';
import { LogLevelBadge } from '../common/Badge';
import { LogSkeleton } from '../common/Skeleton';
import { LogDetailModal } from './LogDetailModal';
import { useDeployments } from '../../hooks/useK8s';
import { useLogSearch } from '../../hooks/useLogs';
import { useClusterStore } from '../../stores/clusterStore';
import { useUIStore } from '../../stores/uiStore';
import { formatShortTimestamp, highlightMatch } from '../../lib/formatters';
import { TIME_RANGES, getTimeRangeLabel, type TimeRange } from '../../lib/tauri';
import { LOG_LEVELS, type LogEntry, type LogSearchFailure } from '../../types/logs';

export function LogSearch() {
  const { context, namespace, deployment: selectedDeployment, setDeployment } = useClusterStore();
  const { openSettings } = useUIStore();
  const { data: deployments = [] } = useDeployments();

  const [keyword, setKeyword] = useState('');
  const [logLevel, setLogLevel] = useState<string>('');
  const [timeRange, setTimeRange] = useState<TimeRange>('1h');
  const [selectedLog, setSelectedLog] = useState<LogEntry | null>(null);

  useEffect(() => setSelectedLog(null), [context, namespace, selectedDeployment]);

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

  const { data, error, isPending, snapshot, search, retry } = useLogSearch();
  const results = data?.results ?? [];
  const allFailed = !!data && data.total_containers > 0 && data.successful_containers === 0;
  const partial = !!data && data.failures.length > 0 && !allFailed;
  const filtersChanged = !!snapshot && (keyword !== snapshot.keyword ||
    logLevel !== snapshot.logLevel || timeRange !== snapshot.timeRange);

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
      setSelectedLog(null);
      search({ context, namespace, deployment: selectedDeployment, keyword, logLevel, timeRange });
    }
  };

  const totalMatches = results.reduce((sum, r) => sum + r.total_matches, 0);
  const uniquePods = new Set(results.map((r) => r.pod_name)).size;

  const handleExport = () => {
    if (!snapshot) return;
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
    a.download = `${snapshot.deployment}-log-search.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="px-6 py-4 border-b border-border">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-lg font-semibold text-text-primary">Log Search</h1>
          <Button variant="ghost" size="icon" onClick={openSettings} title="Settings">
            <Settings className="w-5 h-5" />
          </Button>
        </div>

        {/* Search Form */}
        <form onSubmit={(event) => { event.preventDefault(); handleSearch(); }}>
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
                aria-label="Keyword"
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

          <div className="flex items-center justify-between mt-4 gap-4">
            <span className="text-sm text-amber-600 dark:text-amber-400" role="status">
              {filtersChanged && 'Filters changed. Click Search to apply.'}
            </span>
            <Button
              variant="primary"
              type="submit"
              disabled={!context || !namespace || !selectedDeployment || isPending}
            >
              <Search className="w-4 h-4 mr-2" />
              Search
            </Button>
          </div>
        </form>
      </div>

      {/* Results */}
      <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
        {snapshot && (
          <p className="px-6 pt-4 text-xs text-text-muted">
            Applied filters: {snapshot.keyword ? `Keyword "${snapshot.keyword}"` : 'All keywords'}
            {' · '}{snapshot.logLevel || 'Any level'}{' · '}{getTimeRangeLabel(snapshot.timeRange)}
          </p>
        )}
        {partial && (
          <div role="alert" className="shrink-0 mx-6 mt-4 p-3 border border-amber-500/40 rounded-lg text-sm text-amber-600 dark:text-amber-400">
            <p>Search incomplete: {data.successful_containers} of {data.total_containers} containers searched successfully.</p>
            <FailureDetails failures={data.failures} />
            <Button size="sm" className="mt-2" onClick={retry}>Retry</Button>
          </div>
        )}
        {!snapshot ? (
          <div className="flex flex-col items-center justify-center h-full text-text-muted">
            <Search className="w-16 h-16 mb-4 opacity-30" />
            <p className="text-lg">Search across all pods in a deployment</p>
            <p className="text-sm mt-2">Select a deployment and click Search</p>
          </div>
        ) : isPending ? (
          <LogSkeleton rows={15} />
        ) : error || allFailed ? (
          <div role="alert" className="flex flex-col items-center justify-center px-6 py-8 text-text-primary">
            <p className="text-lg text-red-500">Search failed</p>
            <p className="text-sm mt-2">{error || 'Could not read logs from any container.'}</p>
            {data && <FailureDetails failures={data.failures} />}
            <Button className="mt-4" onClick={retry}>Retry</Button>
          </div>
        ) : data?.total_containers === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-text-muted">
            <p className="text-lg">No pods or containers found</p>
            <p className="text-sm mt-1">This deployment has no regular containers to search.</p>
          </div>
        ) : results.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-text-muted">
            <Search className="w-12 h-12 mb-4 opacity-50" />
            <p className="text-lg">{partial ? 'No matches in successfully searched containers' : 'No results found'}</p>
            <p className="text-sm mt-1">
              {partial ? 'The search is incomplete because some containers could not be read.' : 'Try adjusting your search filters'}
            </p>
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
                    result.entries.map((entry, idx) => {
                      const isError = entry.level === 'ERROR';
                      return (
                        <tr
                          key={`${result.pod_name}/${result.container_name}/${idx}`}
                          className={clsx(
                            'border-b border-border-subtle hover:bg-bg-tertiary/50 transition-colors cursor-pointer',
                            isError && 'bg-red-500/5',
                            entry.level === 'WARN' && 'bg-amber-500/5'
                          )}
                          onClick={() => setSelectedLog({ ...entry, pod_name: result.pod_name, container_name: result.container_name })}
                        >
                          <td className={clsx('px-4 py-2 font-mono text-sm text-text-muted w-24', isError && 'align-top')}>
                            {formatShortTimestamp(entry.timestamp)}
                          </td>
                          <td className={clsx('px-4 py-2', isError && 'align-top')} style={{ width: podColumnWidth }}>
                            <span
                              className="font-mono text-sm text-cyan-500 dark:text-cyan-400 truncate block"
                              style={{ maxWidth: podColumnWidth - 32 }}
                              title={`${result.pod_name} / ${result.container_name}`}
                            >
                              {result.pod_name}
                            </span>
                            <span className="block text-xs text-text-muted truncate" title={result.container_name}>
                              {result.container_name}
                            </span>
                          </td>
                          <td className={clsx('px-4 py-2 w-20', isError && 'align-top')}>
                            <LogLevelBadge level={entry.level} />
                          </td>
                          <td className="px-4 py-2">
                            <HighlightedText text={entry.message} keyword={snapshot.keyword} isError={isError} />
                          </td>
                        </tr>
                      );
                    })
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
            {[...new Set(results.map((r) => r.pod_name))].map((podName) => (
              <span
                key={podName}
                className="inline-flex items-center gap-1 px-2 py-1 bg-bg-tertiary rounded cursor-default"
                title={podName}
              >
                <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                ...{podName.slice(-6)}
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

function FailureDetails({ failures }: { failures: LogSearchFailure[] }) {
  return (
    <details className="mt-2 max-w-full text-sm">
      <summary className="cursor-pointer">Failed containers ({failures.length})</summary>
      <ul className="mt-2 space-y-1 max-h-40 overflow-auto">
        {failures.map((failure) => (
          <li key={`${failure.pod_name}/${failure.container_name}`} className="break-words">
            <span className="font-mono">{failure.pod_name} / {failure.container_name}</span>: {failure.message}
          </li>
        ))}
      </ul>
    </details>
  );
}

function HighlightedText({ text, keyword, isError }: { text: string; keyword: string; isError?: boolean }) {
  const match = highlightMatch(text, keyword);
  const textClass = clsx(
    'font-mono text-sm block',
    isError ? 'line-clamp-2 text-red-500 dark:text-red-400' : 'truncate text-text-secondary'
  );

  if (!match) {
    return <span className={textClass}>{text}</span>;
  }

  return (
    <span className={textClass}>
      {match.before}
      <mark className="bg-amber-500/30 text-text-primary px-0.5 rounded">
        {match.match}
      </mark>
      {match.after}
    </span>
  );
}
