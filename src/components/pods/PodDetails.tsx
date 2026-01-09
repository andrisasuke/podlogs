import { useState } from 'react';
import { clsx } from 'clsx';
import { ChevronDown, Package } from 'lucide-react';
import { StatusBadge, LabelBadge } from '../common/Badge';
import { formatTimestamp } from '../../lib/formatters';
import type { PodDetails as PodDetailsType, ContainerDetails } from '../../types/kubernetes';

interface PodDetailsProps {
  details: PodDetailsType;
}

export function PodDetails({ details }: PodDetailsProps) {
  return (
    <div className="p-4 space-y-6">
      {/* Overview */}
      <Section title="Overview">
        <InfoRow label="Status">
          <StatusBadge status={details.status} />
        </InfoRow>
        <InfoRow label="Node">
          <span className="font-mono text-sm">{details.node}</span>
        </InfoRow>
        <InfoRow label="IP">
          <span className="font-mono text-sm">{details.ip || '-'}</span>
        </InfoRow>
        <InfoRow label="Started">
          <span className="text-sm">{formatTimestamp(details.start_time)}</span>
        </InfoRow>
      </Section>

      {/* Labels */}
      <Section title="Labels">
        <div className="flex flex-wrap gap-2">
          {Object.entries(details.labels).map(([key, value]) => (
            <LabelBadge key={key} label={`${key}=${value}`} />
          ))}
          {Object.keys(details.labels).length === 0 && (
            <span className="text-text-muted text-sm">No labels</span>
          )}
        </div>
      </Section>

      {/* Containers */}
      <Section title={`Containers (${details.containers.length})`}>
        <div className="space-y-3">
          {details.containers.map((container) => (
            <ContainerCard key={container.name} container={container} />
          ))}
        </div>
      </Section>

      {/* Conditions */}
      {details.conditions.length > 0 && (
        <CollapsibleSection title={`Conditions (${details.conditions.length})`}>
          <div className="space-y-2">
            {details.conditions.map((condition, idx) => (
              <div
                key={idx}
                className="flex items-center justify-between py-2 border-b border-border-subtle last:border-0"
              >
                <span className="text-sm text-text-secondary">
                  {condition.condition_type}
                </span>
                <span
                  className={clsx(
                    'text-sm font-medium',
                    condition.status === 'True'
                      ? 'text-emerald-500'
                      : 'text-text-muted'
                  )}
                >
                  {condition.status}
                </span>
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-xs font-medium text-text-muted uppercase tracking-wider mb-3">
        {title}
      </h3>
      {children}
    </div>
  );
}

function CollapsibleSection({
  title,
  children,
  defaultOpen = false,
}: {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
}) {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  return (
    <div>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center gap-2 text-xs font-medium text-text-muted uppercase tracking-wider mb-3 hover:text-text-secondary"
      >
        <ChevronDown
          className={clsx(
            'w-4 h-4 transition-transform',
            isOpen ? 'rotate-0' : '-rotate-90'
          )}
        />
        {title}
      </button>
      {isOpen && children}
    </div>
  );
}

function InfoRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-border-subtle last:border-0">
      <span className="text-sm text-text-secondary">{label}</span>
      {children}
    </div>
  );
}

function ContainerCard({ container }: { container: ContainerDetails }) {
  const [showEnv, setShowEnv] = useState(false);

  return (
    <div className="bg-bg-tertiary rounded-lg p-4">
      {/* Header */}
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-2">
          <Package className="w-4 h-4 text-text-muted" />
          <span className="font-medium text-text-primary">{container.name}</span>
        </div>
        <StatusBadge status={container.state} />
      </div>

      {/* Info */}
      <div className="space-y-2 text-sm">
        <div className="flex items-start gap-2">
          <span className="text-text-muted w-20 shrink-0">Image</span>
          <span className="font-mono text-text-secondary break-all">
            {container.image}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-text-muted w-20">Restarts</span>
          <span className="text-text-secondary">{container.restart_count}</span>
        </div>
        {(container.resources.cpu_request || container.resources.memory_request) && (
          <div className="flex items-center gap-2">
            <span className="text-text-muted w-20">Resources</span>
            <span className="font-mono text-text-secondary">
              CPU: {container.resources.cpu_request || '-'}/
              {container.resources.cpu_limit || '-'} · Mem:{' '}
              {container.resources.memory_request || '-'}/
              {container.resources.memory_limit || '-'}
            </span>
          </div>
        )}
        {container.ports.length > 0 && (
          <div className="flex items-center gap-2">
            <span className="text-text-muted w-20">Ports</span>
            <span className="font-mono text-text-secondary">
              {container.ports
                .map((p) => `${p.container_port}/${p.protocol}`)
                .join(', ')}
            </span>
          </div>
        )}
      </div>

      {/* Environment Variables */}
      {container.env_vars.length > 0 && (
        <div className="mt-4">
          <button
            onClick={() => setShowEnv(!showEnv)}
            className="flex items-center gap-1 text-sm text-text-muted hover:text-text-secondary"
          >
            <ChevronDown
              className={clsx(
                'w-4 h-4 transition-transform',
                showEnv ? 'rotate-0' : '-rotate-90'
              )}
            />
            Environment Variables ({container.env_vars.length})
          </button>
          {showEnv && (
            <div className="mt-2 space-y-1 text-sm">
              {container.env_vars.map((env, idx) => (
                <div
                  key={idx}
                  className="flex items-center gap-2 py-1 px-2 bg-bg-secondary rounded"
                >
                  <span className="font-mono text-cyan-500 dark:text-cyan-400">
                    {env.name}
                  </span>
                  <span className="text-text-secondary flex-1 truncate font-mono">
                    {env.source === 'Secret' ? '<secret>' : env.value}
                  </span>
                  <span className="text-text-muted text-xs">{env.source}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
