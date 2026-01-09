import { clsx } from 'clsx';
import { Layers, Search, Moon, Sun } from 'lucide-react';
import { Dropdown } from '../common/Dropdown';
import { Input } from '../common/Input';
import { Button } from '../common/Button';
import { useClusters, useNamespaces, useDeployments } from '../../hooks/useK8s';
import { useClusterStore } from '../../stores/clusterStore';
import { useUIStore, View } from '../../stores/uiStore';
import { useTheme } from '../../hooks/useTheme';
import { SIDEBAR_WIDTH, SIDEBAR_COLLAPSED_WIDTH } from '../../lib/constants';
import { useState, useMemo, useEffect } from 'react';

export function Sidebar() {
  const { context, namespace, deployment, setContext, setNamespace, setDeployment } =
    useClusterStore();
  const { currentView, setView, sidebarCollapsed } = useUIStore();
  const { isDark, toggleTheme } = useTheme();

  const { data: clusters = [] } = useClusters();
  const { data: namespaces = [] } = useNamespaces();
  const { data: deployments = [] } = useDeployments();

  const [deploymentFilter, setDeploymentFilter] = useState('');

  // Set initial context
  useEffect(() => {
    if (!context && clusters.length > 0) {
      const current = clusters.find((c) => c.is_current);
      setContext(current?.name || clusters[0].name);
    }
  }, [clusters, context, setContext]);

  // Validate namespace exists in current cluster, fallback if not
  useEffect(() => {
    if (namespaces.length > 0 && namespace) {
      const namespaceExists = namespaces.some((n) => n.name === namespace);
      if (!namespaceExists) {
        // Fallback to 'default' if exists, otherwise first namespace
        const defaultNs = namespaces.find((n) => n.name === 'default');
        setNamespace(defaultNs?.name || namespaces[0].name);
      }
    }
  }, [namespaces, namespace, setNamespace]);

  const clusterOptions = useMemo(
    () => clusters.map((c) => ({ value: c.name, label: c.name })),
    [clusters]
  );

  const namespaceOptions = useMemo(
    () => namespaces.map((n) => ({ value: n.name, label: n.name })),
    [namespaces]
  );

  const filteredDeployments = useMemo(() => {
    if (!deploymentFilter) return deployments;
    return deployments.filter((d) =>
      d.name.toLowerCase().includes(deploymentFilter.toLowerCase())
    );
  }, [deployments, deploymentFilter]);

  const navItems: { id: View; label: string; icon: typeof Layers }[] = [
    { id: 'deployments', label: 'Deployments', icon: Layers },
    { id: 'search', label: 'Log Search', icon: Search },
  ];

  if (sidebarCollapsed) {
    return (
      <aside
        className="flex flex-col h-full bg-bg-secondary border-r border-border"
        style={{ width: SIDEBAR_COLLAPSED_WIDTH }}
      >
        {/* Logo */}
        <div className="flex items-center justify-center h-14 border-b border-border">
          <img src="/icon.png" alt="PodLogs" className="w-8 h-8" />
        </div>

        {/* Nav */}
        <nav className="flex-1 py-4">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                onClick={() => setView(item.id)}
                className={clsx(
                  'w-full flex items-center justify-center py-3 transition-colors',
                  currentView === item.id
                    ? 'text-accent bg-accent/10'
                    : 'text-text-secondary hover:bg-bg-tertiary'
                )}
                title={item.label}
              >
                <Icon className="w-5 h-5" />
              </button>
            );
          })}
        </nav>

        {/* Theme toggle */}
        <div className="p-2 border-t border-border">
          <Button variant="ghost" size="icon" onClick={toggleTheme} className="w-full">
            {isDark ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
          </Button>
        </div>
      </aside>
    );
  }

  return (
    <aside
      className="flex flex-col h-full bg-bg-secondary border-r border-border"
      style={{ width: SIDEBAR_WIDTH }}
    >
      {/* Header with logo and theme toggle */}
      <div className="flex items-center justify-between px-4 h-14 border-b border-border">
        <div className="flex items-center gap-2">
          <img src="/icon.png" alt="PodLogs" className="w-7 h-7" />
          <span className="font-semibold text-text-primary">PodLogs</span>
        </div>
        <Button variant="ghost" size="icon" onClick={toggleTheme}>
          {isDark ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
        </Button>
      </div>

      {/* Cluster and Namespace selectors */}
      <div className="px-4 py-4 space-y-4 border-b border-border">
        <div>
          <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
            Cluster
          </label>
          <Dropdown
            options={clusterOptions}
            value={context}
            onChange={setContext}
            placeholder="Select cluster..."
          />
        </div>
        <div>
          <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
            Namespace
          </label>
          <Dropdown
            options={namespaceOptions}
            value={namespace}
            onChange={setNamespace}
            placeholder="Select namespace..."
            disabled={!context}
            searchable
            searchPlaceholder="Search namespaces..."
          />
        </div>
      </div>

      {/* Navigation */}
      <nav className="px-2 py-3 border-b border-border">
        {navItems.map((item) => {
          const Icon = item.icon;
          return (
            <button
              key={item.id}
              onClick={() => setView(item.id)}
              className={clsx(
                'w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm transition-colors',
                currentView === item.id
                  ? 'text-accent bg-accent/10'
                  : 'text-text-secondary hover:bg-bg-tertiary'
              )}
            >
              <Icon className="w-4 h-4" />
              {item.label}
            </button>
          );
        })}
      </nav>

      {/* Deployments list */}
      <div className="flex-1 overflow-hidden flex flex-col">
        <div className="px-4 pt-4 pb-2">
          <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
            Deployments
          </label>
          <Input
            icon
            placeholder="Filter deployments..."
            value={deploymentFilter}
            onChange={(e) => setDeploymentFilter(e.target.value)}
          />
        </div>
        <div className="flex-1 overflow-y-auto px-2 pb-2">
          {filteredDeployments.map((d) => (
            <button
              key={d.name}
              onClick={() => {
                setDeployment(deployment === d.name ? null : d.name);
                setView('pods');
              }}
              className={clsx(
                'w-full flex items-center justify-between px-3 py-2 rounded-md text-sm transition-colors',
                deployment === d.name
                  ? 'text-accent bg-accent/10'
                  : 'text-text-secondary hover:bg-bg-tertiary'
              )}
            >
              <div className="flex items-center gap-2 truncate">
                <span
                  className={clsx(
                    'w-1.5 h-1.5 rounded-full',
                    d.available_replicas === d.replicas
                      ? 'bg-emerald-500'
                      : d.available_replicas > 0
                      ? 'bg-amber-500'
                      : 'bg-red-500'
                  )}
                />
                <span className="truncate">{d.name}</span>
              </div>
              <span className="text-text-muted text-xs ml-2">{d.replicas}</span>
            </button>
          ))}
          {filteredDeployments.length === 0 && (
            <p className="text-text-muted text-sm px-3 py-4">No deployments found</p>
          )}
        </div>
      </div>
    </aside>
  );
}
