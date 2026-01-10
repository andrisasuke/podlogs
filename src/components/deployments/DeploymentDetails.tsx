import { useState } from 'react';
import { clsx } from 'clsx';
import { ChevronDown } from 'lucide-react';
import { LabelBadge } from '../common/Badge';
import { formatTimestamp } from '../../lib/formatters';
import type { DeploymentDetails as DeploymentDetailsType } from '../../types/kubernetes';

interface DeploymentDetailsProps {
  details: DeploymentDetailsType;
}

export function DeploymentDetails({ details }: DeploymentDetailsProps) {
  const isHealthy = details.available_replicas === details.replicas;
  const isPartial = details.available_replicas > 0 && !isHealthy;

  return (
    <div className="p-4 space-y-6">
      {/* Overview */}
      <Section title="Overview">
        <InfoRow label="Status">
          <div className="flex items-center gap-2">
            <span
              className={clsx(
                'w-2 h-2 rounded-full',
                isHealthy ? 'bg-emerald-500' : isPartial ? 'bg-amber-500' : 'bg-red-500'
              )}
            />
            <span
              className={clsx(
                'text-sm font-medium',
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
        </InfoRow>
        <InfoRow label="Replicas">
          <span className="text-sm text-text-secondary">{details.replicas}</span>
        </InfoRow>
        <InfoRow label="Available">
          <span
            className={clsx(
              'text-sm',
              isHealthy ? 'text-emerald-500' : isPartial ? 'text-amber-500' : 'text-red-500'
            )}
          >
            {details.available_replicas}
          </span>
        </InfoRow>
        <InfoRow label="Ready">
          <span className="text-sm text-text-secondary">{details.ready_replicas}</span>
        </InfoRow>
        <InfoRow label="Updated">
          <span className="text-sm text-text-secondary">{details.updated_replicas}</span>
        </InfoRow>
        <InfoRow label="Created">
          <span className="text-sm">{formatTimestamp(details.creation_timestamp)}</span>
        </InfoRow>
      </Section>

      {/* Strategy */}
      <Section title="Strategy">
        <InfoRow label="Type">
          <span className="text-sm text-text-secondary">{details.strategy}</span>
        </InfoRow>
        <InfoRow label="Min Ready Seconds">
          <span className="text-sm text-text-secondary">{details.min_ready_seconds}</span>
        </InfoRow>
        <InfoRow label="Revision History">
          <span className="text-sm text-text-secondary">{details.revision_history_limit}</span>
        </InfoRow>
      </Section>

      {/* Selector */}
      <Section title="Selector">
        <div className="flex flex-wrap gap-2">
          {Object.entries(details.selector).map(([key, value]) => (
            <LabelBadge key={key} label={`${key}=${value}`} />
          ))}
          {Object.keys(details.selector).length === 0 && (
            <span className="text-text-muted text-sm">No selectors</span>
          )}
        </div>
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

      {/* Annotations */}
      {Object.keys(details.annotations).length > 0 && (
        <CollapsibleSection title={`Annotations (${Object.keys(details.annotations).length})`}>
          <div className="space-y-2">
            {Object.entries(details.annotations).map(([key, value]) => (
              <div
                key={key}
                className="py-2 border-b border-border-subtle last:border-0"
              >
                <span className="text-xs text-text-muted block mb-1 font-mono">
                  {key}
                </span>
                <span className="text-sm text-text-secondary break-all">
                  {value}
                </span>
              </div>
            ))}
          </div>
        </CollapsibleSection>
      )}

      {/* Conditions */}
      {details.conditions.length > 0 && (
        <CollapsibleSection title={`Conditions (${details.conditions.length})`} defaultOpen>
          <div className="space-y-3">
            {details.conditions.map((condition, idx) => (
              <div
                key={idx}
                className="py-2 border-b border-border-subtle last:border-0"
              >
                <div className="flex items-center justify-between mb-1">
                  <span className="text-sm font-medium text-text-primary">
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
                {condition.reason && (
                  <span className="text-xs text-text-muted block">
                    Reason: {condition.reason}
                  </span>
                )}
                {condition.message && (
                  <span className="text-xs text-text-secondary block mt-1">
                    {condition.message}
                  </span>
                )}
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
