import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface ClusterState {
  context: string;
  namespace: string;
  deployment: string | null;
  lastNamespaceByCluster: Record<string, string>;

  setContext: (context: string) => void;
  setNamespace: (namespace: string) => void;
  setDeployment: (deployment: string | null) => void;
}

export const useClusterStore = create<ClusterState>()(
  persist(
    (set, get) => ({
      context: '',
      namespace: 'default',
      deployment: null,
      lastNamespaceByCluster: {},

      setContext: (context) => {
        const { lastNamespaceByCluster } = get();
        // Use last visited namespace for this cluster, or 'default' if first time
        const lastNamespace = lastNamespaceByCluster[context] || 'default';
        set({
          context,
          namespace: lastNamespace,
          deployment: null,
        });
      },
      setNamespace: (namespace) => {
        const { context, lastNamespaceByCluster } = get();
        set({
          namespace,
          deployment: null,
          // Remember this namespace for the current cluster
          lastNamespaceByCluster: {
            ...lastNamespaceByCluster,
            [context]: namespace,
          },
        });
      },
      setDeployment: (deployment) => set({ deployment }),
    }),
    {
      name: 'podlogs-cluster',
    }
  )
);
