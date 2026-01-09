import { useState } from 'react';
import { clsx } from 'clsx';
import { Search } from 'lucide-react';
import { Input } from '../common/Input';
import { Button } from '../common/Button';
import { StatusBadge } from '../common/Badge';
import { TableSkeleton } from '../common/Skeleton';
import { usePods } from '../../hooks/useK8s';
import { useClusterStore } from '../../stores/clusterStore';
import { useUIStore } from '../../stores/uiStore';
import type { PodInfo } from '../../types/kubernetes';

export function PodList() {
  const { deployment } = useClusterStore();
  const { selectPod, openLogViewer } = useUIStore();
  const { data: pods = [], isLoading } = usePods(deployment ?? undefined);
  const [searchQuery, setSearchQuery] = useState('');

  const filteredPods = pods.filter((pod) =>
    pod.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border">
        <div className="flex items-center gap-2">
          <h1 className="text-lg font-semibold text-text-primary">Pods</h1>
          {deployment && (
            <span className="text-lg font-semibold text-accent">{deployment}</span>
          )}
        </div>
        <div className="w-64">
          <Input
            icon
            placeholder="Search pods..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
        </div>
      </div>

      {/* Table */}
      <div className="flex-1 overflow-auto">
        {isLoading ? (
          <TableSkeleton rows={8} />
        ) : (
          <table className="w-full">
            <thead className="sticky top-0 bg-bg-primary z-10">
              <tr className="text-left text-xs font-medium text-text-muted uppercase tracking-wider border-b border-border">
                <th className="px-6 py-3">Name</th>
                <th className="px-4 py-3 w-28">Status</th>
                <th className="px-4 py-3 w-20 text-center">Ready</th>
                <th className="px-4 py-3 w-24 text-center">Restarts</th>
                <th className="px-4 py-3 w-20 text-right">Age</th>
                <th className="px-4 py-3 w-28 text-right">IP</th>
                <th className="px-6 py-3 w-36 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredPods.map((pod) => (
                <PodRow
                  key={pod.name}
                  pod={pod}
                  onViewLogs={() => openLogViewer(pod.name)}
                  onViewInfo={() => selectPod(pod.name)}
                />
              ))}
            </tbody>
          </table>
        )}

        {!isLoading && filteredPods.length === 0 && (
          <div className="flex flex-col items-center justify-center h-64 text-text-muted">
            <Search className="w-12 h-12 mb-4 opacity-50" />
            <p className="text-lg">No pods found</p>
            {searchQuery && (
              <p className="text-sm mt-1">
                Try adjusting your search query
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

interface PodRowProps {
  pod: PodInfo;
  onViewLogs: () => void;
  onViewInfo: () => void;
}

function PodRow({ pod, onViewLogs, onViewInfo }: PodRowProps) {
  const isTerminating = pod.status === 'Terminating';
  const isCreating = pod.status === 'ContainerCreating';
  const isEvicted = pod.status === 'Evicted';
  const isDisabled = isTerminating || isCreating || isEvicted;

  return (
    <tr className="border-b border-border-subtle hover:bg-bg-tertiary/50 transition-colors">
      <td className="px-6 py-3">
        <span className="font-mono text-sm text-text-primary">{pod.name}</span>
      </td>
      <td className="px-4 py-3">
        <StatusBadge status={pod.status} ready={pod.ready} />
      </td>
      <td className="px-4 py-3 text-center">
        <span className="text-sm text-text-secondary">{pod.ready}</span>
      </td>
      <td className="px-4 py-3 text-center">
        <span
          className={clsx(
            'text-sm',
            pod.restarts > 0 ? 'text-red-500 dark:text-red-400' : 'text-text-secondary'
          )}
        >
          {pod.restarts}
        </span>
      </td>
      <td className="px-4 py-3 text-right">
        <span className="text-sm text-text-secondary">{pod.age}</span>
      </td>
      <td className="px-4 py-3 text-right">
        <span className="font-mono text-sm text-text-secondary">
          {pod.ip || '-'}
        </span>
      </td>
      <td className="px-6 py-3 text-right">
        <div className="flex items-center justify-end gap-2">
          <Button
            variant="primary"
            size="sm"
            onClick={onViewLogs}
            disabled={isDisabled}
            title={isDisabled ? `Pod is ${pod.status.toLowerCase()}` : undefined}
          >
            Logs
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={onViewInfo}
            disabled={isDisabled}
            title={isDisabled ? `Pod is ${pod.status.toLowerCase()}` : undefined}
          >
            Info
          </Button>
        </div>
      </td>
    </tr>
  );
}
