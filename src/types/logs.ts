export interface LogEntry {
  timestamp: string | null;
  level: LogLevel | null;
  message: string;
  raw: string;
  is_json: boolean;
  pod_name: string;
  container_name: string;
}

export interface LogSearchResult {
  pod_name: string;
  container_name: string;
  total_matches: number;
  entries: LogEntry[];
}

export interface LogSearchFailure {
  pod_name: string;
  container_name: string;
  message: string;
}

export interface LogSearchResponse {
  results: LogSearchResult[];
  failures: LogSearchFailure[];
  total_containers: number;
  successful_containers: number;
}

export type LogLevel = 'ERROR' | 'WARN' | 'INFO' | 'DEBUG';

export const LOG_LEVELS: LogLevel[] = ['ERROR', 'WARN', 'INFO', 'DEBUG'];
