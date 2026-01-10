import { useState } from 'react';
import { clsx } from 'clsx';
import { Layers } from 'lucide-react';
import { Input } from '../common/Input';
import { Button } from '../common/Button';
import { TableSkeleton } from '../common/Skeleton';
import { useDeployments } from '../../hooks/useK8s';
import { useClusterStore } from '../../stores/clusterStore';
import { useUIStore } from '../../stores/uiStore';
import type { DeploymentInfo } from '../../types/kubernetes';

export function DeploymentList() {
  const { data: deployments = [], isLoading } = useDeployments();
  const { setDeployment } = useClusterStore();
  const { setView, selectDeploymentInfo } = useUIStore();
  const [searchQuery, setSearchQuery] = useState('');

  const filteredDeployments = deployments.filter((d) =>
    d.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleViewInfo = (name: string) => {
    selectDeploymentInfo(name);
  };

  const handleRowClick = (name: string) => {
    setDeployment(name);
    setView('pods');
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border">
        <h1 className="text-lg font-semibold text-text-primary">Deployments</h1>
        <div className="w-64">
          <Input
            icon
            placeholder="Search deployments..."
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
                <th className="px-4 py-3 w-32 text-center">Replicas</th>
                <th className="px-4 py-3 w-32 text-center">Available</th>
                <th className="px-4 py-3 w-32 text-center">Ready</th>
                <th className="px-4 py-3 w-32">Status</th>
                <th className="px-6 py-3 w-36 text-right">Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredDeployments.map((deployment) => (
                <DeploymentRow
                  key={deployment.name}
                  deployment={deployment}
                  onClick={() => handleRowClick(deployment.name)}
                  onViewInfo={() => handleViewInfo(deployment.name)}
                />
              ))}
            </tbody>
          </table>
        )}

        {!isLoading && filteredDeployments.length === 0 && (
          <div className="flex flex-col items-center justify-center h-64 text-text-muted">
            <Layers className="w-12 h-12 mb-4 opacity-50" />
            <p className="text-lg">No deployments found</p>
          </div>
        )}
      </div>
    </div>
  );
}

interface DeploymentRowProps {
  deployment: DeploymentInfo;
  onClick: () => void;
  onViewInfo: () => void;
}

function DeploymentRow({ deployment, onClick, onViewInfo }: DeploymentRowProps) {
  const isHealthy = deployment.available_replicas === deployment.replicas;
  const isPartial = deployment.available_replicas > 0 && !isHealthy;

  return (
    <tr
      className="border-b border-border-subtle hover:bg-bg-tertiary/50 transition-colors cursor-pointer"
      onClick={onClick}
    >
      <td className="px-6 py-3">
        <div className="flex items-center gap-2">
          <Layers className="w-4 h-4 text-text-muted" />
          <span className="font-medium text-text-primary">{deployment.name}</span>
        </div>
      </td>
      <td className="px-4 py-3 text-center">
        <span className="text-sm text-text-secondary">{deployment.replicas}</span>
      </td>
      <td className="px-4 py-3 text-center">
        <span
          className={clsx(
            'text-sm',
            isHealthy ? 'text-emerald-500' : isPartial ? 'text-amber-500' : 'text-red-500'
          )}
        >
          {deployment.available_replicas}
        </span>
      </td>
      <td className="px-4 py-3 text-center">
        <span className="text-sm text-text-secondary">{deployment.ready_replicas}</span>
      </td>
      <td className="px-4 py-3">
        <div className="flex items-center gap-2">
          <span
            className={clsx(
              'w-2 h-2 rounded-full',
              isHealthy ? 'bg-emerald-500' : isPartial ? 'bg-amber-500' : 'bg-red-500'
            )}
          />
          <span
            className={clsx(
              'text-sm',
              isHealthy
                ? 'text-emerald-500'
                : isPartial
                ? 'text-amber-500'
                : 'text-red-500'
            )}
          >
            {isHealthy ? 'Healthy' : isPartial ? 'Degraded' : 'Unhealthy'}
          </span>
        </div>
      </td>
      <td className="px-6 py-3 text-right">
        <Button
          variant="secondary"
          size="sm"
          onClick={(e) => {
            e.stopPropagation();
            onViewInfo();
          }}
        >
          Info
        </Button>
      </td>
    </tr>
  );
}
