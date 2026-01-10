export interface ClusterInfo {
  name: string;
  server: string;
  is_current: boolean;
}

export interface NamespaceInfo {
  name: string;
  status: string;
}

export interface DeploymentInfo {
  name: string;
  namespace: string;
  replicas: number;
  available_replicas: number;
  ready_replicas: number;
}

export interface DeploymentDetails {
  name: string;
  namespace: string;
  replicas: number;
  available_replicas: number;
  ready_replicas: number;
  updated_replicas: number;
  strategy: string;
  min_ready_seconds: number;
  revision_history_limit: number;
  creation_timestamp: string | null;
  labels: Record<string, string>;
  annotations: Record<string, string>;
  selector: Record<string, string>;
  conditions: DeploymentCondition[];
}

export interface DeploymentCondition {
  condition_type: string;
  status: string;
  reason: string | null;
  message: string | null;
  last_update_time: string | null;
  last_transition_time: string | null;
}

export interface PodInfo {
  name: string;
  namespace: string;
  status: string;
  ready: string;
  restarts: number;
  age: string;
  ip: string;
  node: string;
  containers: string[];
}

export interface PodDetails {
  name: string;
  namespace: string;
  status: string;
  node: string;
  ip: string;
  start_time: string | null;
  labels: Record<string, string>;
  annotations: Record<string, string>;
  containers: ContainerDetails[];
  conditions: PodCondition[];
}

export interface ContainerDetails {
  name: string;
  image: string;
  ready: boolean;
  restart_count: number;
  state: string;
  env_vars: EnvVar[];
  ports: ContainerPort[];
  resources: ResourceRequirements;
}

export interface EnvVar {
  name: string;
  value: string;
  source: 'Direct' | 'Secret' | 'ConfigMap' | 'FieldRef' | 'Reference' | 'Unknown';
}

export interface ContainerPort {
  name: string | null;
  container_port: number;
  protocol: string;
}

export interface ResourceRequirements {
  cpu_request: string | null;
  cpu_limit: string | null;
  memory_request: string | null;
  memory_limit: string | null;
}

export interface PodCondition {
  condition_type: string;
  status: string;
  reason: string | null;
  message: string | null;
  last_transition_time: string | null;
}

export type PodStatus = 'Running' | 'Pending' | 'Succeeded' | 'Failed' | 'Unknown' | 'CrashLoopBackOff' | 'ImagePullBackOff' | 'ErrImagePull' | 'ContainerCreating' | 'Terminating' | 'Evicted' | 'UnexpectedAdmissionError' | 'ContainerStatusUnknown';
