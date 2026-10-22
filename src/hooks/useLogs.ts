import { useEffect, useRef, useState } from 'react';
import { useMutation, useQuery } from '@tanstack/react-query';
import * as k8s from '../lib/tauri';
import { useClusterStore } from '../stores/clusterStore';
import { useUIStore } from '../stores/uiStore';
import type { TimeRange } from '../lib/tauri';
import type { LogSearchResponse } from '../types/logs';

export function usePodLogs(
  podName: string | null,
  options: {
    container?: string;
    timeRange: TimeRange;
    enabled?: boolean;
  }
) {
  const { context, namespace } = useClusterStore();
  const refreshInterval = useUIStore((state) => state.refreshInterval);

  return useQuery({
    queryKey: ['pod-logs', context, namespace, podName, options.container, options.timeRange],
    queryFn: () =>
      k8s.getPodLogs(context, namespace, podName!, {
        container: options.container,
        sinceSeconds: k8s.TIME_RANGES[options.timeRange],
      }),
    enabled: options.enabled !== false && !!context && !!namespace && !!podName,
    refetchInterval: refreshInterval,
  });
}

export interface LogSearchSnapshot {
  context: string;
  namespace: string;
  deployment: string;
  keyword: string;
  logLevel: string;
  timeRange: TimeRange;
}

type SearchState =
  | { status: 'idle' }
  | { status: 'pending'; snapshot: LogSearchSnapshot }
  | { status: 'success'; snapshot: LogSearchSnapshot; data: LogSearchResponse }
  | { status: 'error'; snapshot: LogSearchSnapshot; error: string };

function isCurrentScope(snapshot: LogSearchSnapshot) {
  const { context, namespace, deployment } = useClusterStore.getState();
  return snapshot.context === context && snapshot.namespace === namespace &&
    snapshot.deployment === deployment;
}

export function useLogSearch() {
  const { context, namespace, deployment } = useClusterStore();
  const [state, setState] = useState<SearchState>({ status: 'idle' });
  const requestId = useRef(0);
  const pending = useRef(false);
  const { mutateAsync, reset } = useMutation<LogSearchResponse, unknown, LogSearchSnapshot>({
    mutationFn: (snapshot) =>
      k8s.searchDeploymentLogs(snapshot.context, snapshot.namespace, snapshot.deployment, {
        keyword: snapshot.keyword || undefined,
        logLevel: snapshot.logLevel || undefined,
        sinceSeconds: k8s.TIME_RANGES[snapshot.timeRange],
      }),
    retry: false,
  });

  useEffect(() => {
    requestId.current += 1;
    pending.current = false;
    setState({ status: 'idle' });
    reset();
    return () => {
      // Invalidate in-flight responses on scope change or unmount.
      requestId.current += 1;
      pending.current = false;
    };
  }, [context, namespace, deployment, reset]);

  const search = (snapshot: LogSearchSnapshot) => {
    if (pending.current || !snapshot.context || !snapshot.namespace ||
        !snapshot.deployment || !isCurrentScope(snapshot)) return;

    const id = ++requestId.current;
    pending.current = true;
    setState({ status: 'pending', snapshot });
    void mutateAsync(snapshot).then(
      (data) => {
        if (requestId.current !== id || !isCurrentScope(snapshot)) return;
        pending.current = false;
        setState({ status: 'success', snapshot, data });
      },
      (error: unknown) => {
        if (requestId.current !== id || !isCurrentScope(snapshot)) return;
        pending.current = false;
        setState({
          status: 'error', snapshot,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    );
  };

  // Hide the previous scope immediately, before the reset effect runs.
  const visibleState: SearchState = 'snapshot' in state && !isCurrentScope(state.snapshot)
    ? { status: 'idle' }
    : state;
  const snapshot = 'snapshot' in visibleState ? visibleState.snapshot : undefined;

  return {
    search,
    retry: () => { if (snapshot) search(snapshot); },
    snapshot,
    data: visibleState.status === 'success' ? visibleState.data : undefined,
    error: visibleState.status === 'error' ? visibleState.error : undefined,
    isPending: visibleState.status === 'pending',
  };
}
