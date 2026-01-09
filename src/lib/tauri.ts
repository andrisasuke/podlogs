import { invoke } from '@tauri-apps/api/core';
import type {
  ClusterInfo,
  NamespaceInfo,
  DeploymentInfo,
  PodInfo,
  PodDetails,
} from '../types/kubernetes';
import type { LogEntry, LogSearchResult } from '../types/logs';

// Timeout for K8s API calls (30 seconds)
const API_TIMEOUT = 30000;

// Wrapper to add timeout to invoke calls
async function invokeWithTimeout<T>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  const timeoutPromise = new Promise<never>((_, reject) => {
    setTimeout(() => {
      reject(new Error(`Connection timed out after ${API_TIMEOUT / 1000} seconds`));
    }, API_TIMEOUT);
  });

  return Promise.race([
    invoke<T>(cmd, args),
    timeoutPromise,
  ]);
}

// ============================================
// Cluster Operations
// ============================================

export async function getClusters(): Promise<ClusterInfo[]> {
  // getClusters reads from kubeconfig file, no network timeout needed
  return invoke<ClusterInfo[]>('get_clusters');
}

// ============================================
// Namespace Operations
// ============================================

export async function getNamespaces(context: string): Promise<NamespaceInfo[]> {
  return invokeWithTimeout<NamespaceInfo[]>('get_namespaces', { context });
}

// ============================================
// Deployment Operations
// ============================================

export async function getDeployments(
  context: string,
  namespace: string
): Promise<DeploymentInfo[]> {
  return invokeWithTimeout<DeploymentInfo[]>('get_deployments', { context, namespace });
}

// ============================================
// Pod Operations
// ============================================

export async function getPods(
  context: string,
  namespace: string,
  deployment?: string
): Promise<PodInfo[]> {
  return invokeWithTimeout<PodInfo[]>('get_pods', { context, namespace, deployment });
}

export async function getPodDetails(
  context: string,
  namespace: string,
  podName: string
): Promise<PodDetails> {
  return invokeWithTimeout<PodDetails>('get_pod_details', {
    context,
    namespace,
    podName,
  });
}

// ============================================
// Log Operations
// ============================================

export async function getPodLogs(
  context: string,
  namespace: string,
  podName: string,
  options?: {
    container?: string;
    sinceSeconds?: number;
    tailLines?: number;
  }
): Promise<LogEntry[]> {
  return invokeWithTimeout<LogEntry[]>('get_pod_logs', {
    context,
    namespace,
    podName,
    container: options?.container,
    sinceSeconds: options?.sinceSeconds,
    tailLines: options?.tailLines,
  });
}

export async function searchDeploymentLogs(
  context: string,
  namespace: string,
  deployment: string,
  options?: {
    keyword?: string;
    logLevel?: string;
    sinceSeconds?: number;
  }
): Promise<LogSearchResult[]> {
  return invokeWithTimeout<LogSearchResult[]>('search_deployment_logs', {
    context,
    namespace,
    deployment,
    keyword: options?.keyword,
    logLevel: options?.logLevel,
    sinceSeconds: options?.sinceSeconds,
  });
}

// ============================================
// Time Range Helpers
// ============================================

export const TIME_RANGES = {
  '15m': 15 * 60,
  '30m': 30 * 60,
  '1h': 60 * 60,
  '3h': 3 * 60 * 60,
  '6h': 6 * 60 * 60,
  '12h': 12 * 60 * 60,
  '24h': 24 * 60 * 60,
  '48h': 48 * 60 * 60,
} as const;

export type TimeRange = keyof typeof TIME_RANGES;

export function getTimeRangeLabel(range: TimeRange): string {
  const labels: Record<TimeRange, string> = {
    '15m': 'Last 15 minutes',
    '30m': 'Last 30 minutes',
    '1h': 'Last 1 hour',
    '3h': 'Last 3 hours',
    '6h': 'Last 6 hours',
    '12h': 'Last 12 hours',
    '24h': 'Last 24 hours',
    '48h': 'Last 48 hours',
  };
  return labels[range];
}
