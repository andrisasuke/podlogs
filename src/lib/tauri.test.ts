import { invoke } from '@tauri-apps/api/core';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { searchDeploymentLogs } from './tauri';
import type { LogSearchResponse } from '../types/logs';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);
const response: LogSearchResponse = {
  results: [], failures: [{ pod_name: 'queue-a', container_name: 'master', message: 'Forbidden' }],
  total_containers: 1, successful_containers: 0,
};

beforeEach(() => {
  vi.useFakeTimers();
  mockedInvoke.mockReset();
});
afterEach(() => vi.useRealTimers());

it('preserves the response contract and cancels the timeout timer after success', async () => {
  mockedInvoke.mockResolvedValue(response);
  await expect(searchDeploymentLogs('fixture', 'spp', 'queue', {
    keyword: 'forecast sync', logLevel: 'ERROR', sinceSeconds: 10800,
  })).resolves.toEqual(response);
  expect(mockedInvoke).toHaveBeenCalledWith('search_deployment_logs', {
    context: 'fixture', namespace: 'spp', deployment: 'queue',
    keyword: 'forecast sync', logLevel: 'ERROR', sinceSeconds: 10800,
  });
  expect(vi.getTimerCount()).toBe(0);
});

it('preserves global failures and cancels the timeout timer', async () => {
  mockedInvoke.mockRejectedValue('Config error: unknown context');
  await expect(searchDeploymentLogs('fixture', 'spp', 'queue')).rejects.toBe('Config error: unknown context');
  expect(vi.getTimerCount()).toBe(0);
});

it('rejects a pending command at 30 seconds instead of returning empty results', async () => {
  mockedInvoke.mockReturnValue(new Promise(() => {}));
  const request = searchDeploymentLogs('fixture', 'spp', 'queue');
  const assertion = expect(request).rejects.toThrow('Connection timed out after 30 seconds');
  await vi.advanceTimersByTimeAsync(29999);
  expect(vi.getTimerCount()).toBe(1);
  await vi.advanceTimersByTimeAsync(1);
  await assertion;
  expect(mockedInvoke).toHaveBeenCalledTimes(1);
  expect(vi.getTimerCount()).toBe(0);
});
