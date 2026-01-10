import { useQuery, useQueryClient } from '@tanstack/react-query';
import * as k8s from '../lib/tauri';
import { useClusterStore } from '../stores/clusterStore';
import { useErrorStore } from '../stores/errorStore';
import { REFETCH_INTERVAL } from '../lib/constants';

// Get error setter for use in query callbacks
const getErrorSetter = () => useErrorStore.getState().setError;

// Helper to report error only if no cached data exists (initial load failure)
function reportErrorIfNoCache<T>(
  queryClient: ReturnType<typeof useQueryClient>,
  queryKey: unknown[],
  error: Error
) {
  const cachedData = queryClient.getQueryData<T>(queryKey);
  // Only show error dialog if there's no cached data (initial load)
  if (!cachedData || (Array.isArray(cachedData) && cachedData.length === 0)) {
    getErrorSetter()(error);
  }
}

export function useClusters() {
  const queryClient = useQueryClient();
  const queryKey = ['clusters'];

  return useQuery({
    queryKey,
    queryFn: async () => {
      try {
        return await k8s.getClusters();
      } catch (error) {
        reportErrorIfNoCache(queryClient, queryKey, error as Error);
        throw error;
      }
    },
    staleTime: Infinity,
    retry: 0,
  });
}

export function useNamespaces() {
  const queryClient = useQueryClient();
  const { context } = useClusterStore();
  const queryKey = ['namespaces', context];

  return useQuery({
    queryKey,
    queryFn: async () => {
      try {
        return await k8s.getNamespaces(context);
      } catch (error) {
        reportErrorIfNoCache(queryClient, queryKey, error as Error);
        throw error;
      }
    },
    enabled: !!context,
    staleTime: 60000,
    retry: 0,
  });
}

export function useDeployments() {
  const queryClient = useQueryClient();
  const { context, namespace } = useClusterStore();
  const queryKey = ['deployments', context, namespace];

  return useQuery({
    queryKey,
    queryFn: async () => {
      try {
        return await k8s.getDeployments(context, namespace);
      } catch (error) {
        reportErrorIfNoCache(queryClient, queryKey, error as Error);
        throw error;
      }
    },
    enabled: !!context && !!namespace,
    staleTime: 30000,
    refetchInterval: REFETCH_INTERVAL,
    retry: 0,
  });
}

export function usePods(deployment?: string) {
  const queryClient = useQueryClient();
  const { context, namespace } = useClusterStore();
  const queryKey = ['pods', context, namespace, deployment];

  return useQuery({
    queryKey,
    queryFn: async () => {
      try {
        return await k8s.getPods(context, namespace, deployment);
      } catch (error) {
        reportErrorIfNoCache(queryClient, queryKey, error as Error);
        throw error;
      }
    },
    enabled: !!context && !!namespace,
    refetchInterval: REFETCH_INTERVAL,
    retry: 0,
  });
}

export function usePodDetails(podName: string | null) {
  const queryClient = useQueryClient();
  const { context, namespace } = useClusterStore();
  const queryKey = ['pod-details', context, namespace, podName];

  return useQuery({
    queryKey,
    queryFn: async () => {
      try {
        return await k8s.getPodDetails(context, namespace, podName!);
      } catch (error) {
        reportErrorIfNoCache(queryClient, queryKey, error as Error);
        throw error;
      }
    },
    enabled: !!context && !!namespace && !!podName,
    retry: 0,
  });
}

export function useDeploymentDetails(deploymentName: string | null) {
  const queryClient = useQueryClient();
  const { context, namespace } = useClusterStore();
  const queryKey = ['deployment-details', context, namespace, deploymentName];

  return useQuery({
    queryKey,
    queryFn: async () => {
      try {
        return await k8s.getDeploymentDetails(context, namespace, deploymentName!);
      } catch (error) {
        reportErrorIfNoCache(queryClient, queryKey, error as Error);
        throw error;
      }
    },
    enabled: !!context && !!namespace && !!deploymentName,
    retry: 0,
  });
}
