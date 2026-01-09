import { useQuery } from '@tanstack/react-query';
import * as k8s from '../lib/tauri';
import { useClusterStore } from '../stores/clusterStore';
import type { TimeRange } from '../lib/tauri';
import { LOG_REFETCH_INTERVAL } from '../lib/constants';

export function usePodLogs(
  podName: string | null,
  options: {
    container?: string;
    timeRange: TimeRange;
    enabled?: boolean;
  }
) {
  const { context, namespace } = useClusterStore();

  return useQuery({
    queryKey: ['pod-logs', context, namespace, podName, options.container, options.timeRange],
    queryFn: () =>
      k8s.getPodLogs(context, namespace, podName!, {
        container: options.container,
        sinceSeconds: k8s.TIME_RANGES[options.timeRange],
      }),
    enabled: options.enabled !== false && !!context && !!namespace && !!podName,
    refetchInterval: LOG_REFETCH_INTERVAL,
  });
}

export function useLogSearch(
  deployment: string | null,
  filters: {
    keyword?: string;
    logLevel?: string;
    timeRange: TimeRange;
  },
  enabled: boolean = false
) {
  const { context, namespace } = useClusterStore();

  return useQuery({
    queryKey: ['log-search', context, namespace, deployment, filters],
    queryFn: () =>
      k8s.searchDeploymentLogs(context, namespace, deployment!, {
        keyword: filters.keyword || undefined,
        logLevel: filters.logLevel || undefined, // Convert empty string to undefined for "Any"
        sinceSeconds: k8s.TIME_RANGES[filters.timeRange],
      }),
    enabled: enabled && !!context && !!namespace && !!deployment,
  });
}
