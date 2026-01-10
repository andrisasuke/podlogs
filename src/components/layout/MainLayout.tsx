import { useEffect, useRef } from 'react';
import { Sidebar } from './Sidebar';
import { PodList } from '../pods/PodList';
import { PodDetails } from '../pods/PodDetails';
import { DeploymentDetails } from '../deployments/DeploymentDetails';
import { LogViewer } from '../logs/LogViewer';
import { LogSearch } from '../logs/LogSearch';
import { DeploymentList } from '../deployments/DeploymentList';
import { StatusBar } from './StatusBar';
import { Drawer } from '../common/Drawer';
import { ConnectionError } from '../common/ErrorDialog';
import { useUIStore } from '../../stores/uiStore';
import { useClusterStore } from '../../stores/clusterStore';
import { usePodDetails, useDeploymentDetails } from '../../hooks/useK8s';
import { useErrorStore } from '../../stores/errorStore';
import { useQueryClient } from '@tanstack/react-query';

export function MainLayout() {
  const { currentView, selectedPod, selectedDeploymentInfo, drawerOpen, drawerType, setDrawerOpen, logViewerPod, selectPod, setView } = useUIStore();
  const { context, namespace } = useClusterStore();
  const { data: podDetails } = usePodDetails(selectedPod);
  const { data: deploymentDetails } = useDeploymentDetails(selectedDeploymentInfo);
  const queryClient = useQueryClient();

  // Track previous namespace/context to detect changes
  const prevNamespaceRef = useRef(namespace);
  const prevContextRef = useRef(context);

  // Use global error store
  const { error: connectionError, clearError, startTransition, endTransition } = useErrorStore();

  // Close log viewer and drawer when namespace or context changes
  useEffect(() => {
    if (prevNamespaceRef.current !== namespace || prevContextRef.current !== context) {
      // Start transition to ignore errors during switch
      startTransition();

      // Cancel any in-flight queries for old namespace/context
      queryClient.cancelQueries({ queryKey: ['pod-logs'] });
      queryClient.cancelQueries({ queryKey: ['pod-details'] });

      // Reset view to deployments (deployment is cleared on namespace change)
      if (currentView === 'logs' || currentView === 'pods') {
        setView('deployments');
      }

      // Close pod details drawer
      if (selectedPod) {
        selectPod(null);
      }

      prevNamespaceRef.current = namespace;
      prevContextRef.current = context;

      // End transition after a delay to allow pending errors to be ignored
      const timer = setTimeout(() => endTransition(), 1000);
      return () => clearTimeout(timer);
    }
    prevNamespaceRef.current = namespace;
    prevContextRef.current = context;
  }, [namespace, context, currentView, selectedPod, setView, selectPod, queryClient, startTransition, endTransition]);

  const handleRetry = async () => {
    clearError();
    // Reset queries to clear error state and cached data
    await queryClient.resetQueries({ queryKey: ['clusters'] });
    await queryClient.resetQueries({ queryKey: ['namespaces'] });
    await queryClient.resetQueries({ queryKey: ['deployments'] });
    await queryClient.resetQueries({ queryKey: ['pods'] });
    // Refetch all queries immediately
    queryClient.refetchQueries({ queryKey: ['clusters'] });
    queryClient.refetchQueries({ queryKey: ['namespaces'] });
    queryClient.refetchQueries({ queryKey: ['deployments'] });
    queryClient.refetchQueries({ queryKey: ['pods'] });
  };

  const renderContent = () => {
    switch (currentView) {
      case 'pods':
        return <PodList />;
      case 'deployments':
        return <DeploymentList />;
      case 'logs':
        return <LogViewer podName={logViewerPod} />;
      case 'search':
        return <LogSearch />;
      default:
        return <PodList />;
    }
  };

  return (
    <div className="flex h-screen overflow-hidden bg-bg-primary">
      <Sidebar />

      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Main content */}
        <main className="flex-1 overflow-hidden">{renderContent()}</main>

        {/* Status bar */}
        <StatusBar />
      </div>

      {/* Pod details drawer */}
      <Drawer
        isOpen={drawerOpen && drawerType === 'pod'}
        onClose={() => setDrawerOpen(false)}
        title={selectedPod || ''}
      >
        {podDetails && <PodDetails details={podDetails} />}
      </Drawer>

      {/* Deployment details drawer */}
      <Drawer
        isOpen={drawerOpen && drawerType === 'deployment'}
        onClose={() => setDrawerOpen(false)}
        title={selectedDeploymentInfo || ''}
      >
        {deploymentDetails && <DeploymentDetails details={deploymentDetails} />}
      </Drawer>

      {/* Connection Error Dialog */}
      <ConnectionError
        error={connectionError}
        onDismiss={clearError}
        onRetry={handleRetry}
      />
    </div>
  );
}
