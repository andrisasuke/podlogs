import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useClusterStore } from '../../stores/clusterStore';
import type { LogEntry, LogSearchResponse } from '../../types/logs';
import { LogSearch } from './LogSearch';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('../../hooks/useK8s', () => ({
  useDeployments: () => ({ data: [{ name: 'queue' }, { name: 'other' }] }),
}));

const mockedInvoke = vi.mocked(invoke);
const emptyResponse: LogSearchResponse = {
  results: [], failures: [], total_containers: 1, successful_containers: 1,
};
const entry: LogEntry = {
  timestamp: '2026-09-30T12:00:06Z', level: 'ERROR',
  message: 'Forecast sync failed for warehouse jk01',
  raw: 'ERROR Forecast sync failed for warehouse jk01', is_json: false,
  pod_name: 'queue-a', container_name: 'master',
};
const matchedResponse: LogSearchResponse = {
  ...emptyResponse,
  results: [{ pod_name: entry.pod_name, container_name: entry.container_name, total_matches: 1, entries: [entry] }],
};
const failure = { pod_name: 'queue-b', container_name: 'sidecar', message: 'Access denied' };
let queryClient: QueryClient;

function renderSearch() {
  queryClient = new QueryClient({ defaultOptions: { mutations: { retry: 2 } } });
  return render(<QueryClientProvider client={queryClient}><LogSearch /></QueryClientProvider>);
}

function deferred() {
  let resolve!: (response: LogSearchResponse) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<LogSearchResponse>((res, rej) => { resolve = res; reject = rej; });
  return { promise, resolve, reject };
}

beforeEach(() => {
  mockedInvoke.mockReset();
  mockedInvoke.mockResolvedValue(emptyResponse);
  useClusterStore.setState({ context: 'fixture', namespace: 'spp', deployment: 'queue' });
});

afterEach(() => {
  queryClient?.clear();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('manual log search', () => {
  it('submits every click and Enter, including identical parameters and empty keyword', async () => {
    const user = userEvent.setup();
    renderSearch();
    expect(mockedInvoke).not.toHaveBeenCalled();
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByText('No results found');
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledTimes(2));
    await screen.findByText('No results found');
    await user.type(screen.getByRole('textbox', { name: 'Keyword' }), '{Enter}');
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledTimes(3));
    for (const call of mockedInvoke.mock.calls) {
      expect(call).toEqual(['search_deployment_logs', {
        context: 'fixture', namespace: 'spp', deployment: 'queue',
        keyword: undefined, logLevel: undefined, sinceSeconds: 3600,
      }]);
    }
  });

  it('edits filters without requesting and keeps the applied highlight and export', async () => {
    const user = userEvent.setup();
    mockedInvoke.mockResolvedValue(matchedResponse);
    const createObjectURL = vi.fn<(blob: Blob) => string>(() => 'blob:fixture');
    const revokeObjectURL = vi.fn();
    vi.stubGlobal('URL', class extends URL {
      static createObjectURL = createObjectURL;
      static revokeObjectURL = revokeObjectURL;
    });
    const download = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    renderSearch();
    await user.type(screen.getByRole('textbox', { name: 'Keyword' }), 'forecast');
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByRole('table');
    await user.clear(screen.getByRole('textbox', { name: 'Keyword' }));
    await user.type(screen.getByRole('textbox', { name: 'Keyword' }), 'heartbeat');
    await user.click(screen.getByRole('button', { name: 'Any' }));
    // The result table also contains ERROR, so select the dropdown option by role.
    await user.click(screen.getByRole('button', { name: 'ERROR' }));
    await user.click(screen.getByRole('button', { name: 'Last 1 hour' }));
    await user.click(screen.getByRole('button', { name: 'Last 3 hours' }));
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    expect(screen.getByText('Filters changed. Click Search to apply.')).toBeInTheDocument();
    expect(screen.getByText('Forecast', { selector: 'mark' })).toBeInTheDocument();
    expect(screen.getByText(/Applied filters:/)).toHaveTextContent('Keyword "forecast" · Any level · Last 1 hour');
    await user.click(screen.getByRole('button', { name: 'Export CSV' }));
    expect(createObjectURL).toHaveBeenCalledTimes(1);
    expect((download.mock.contexts[0] as HTMLAnchorElement).download).toBe('queue-log-search.csv');
    expect(createObjectURL.mock.calls[0][0]).toBeInstanceOf(Blob);
    expect(revokeObjectURL).toHaveBeenCalledWith('blob:fixture');
    download.mockRestore();
    vi.unstubAllGlobals();

    await user.click(screen.getByRole('button', { name: 'Search' }));
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledTimes(2));
    expect(mockedInvoke).toHaveBeenLastCalledWith('search_deployment_logs', {
      context: 'fixture', namespace: 'spp', deployment: 'queue',
      keyword: 'heartbeat', logLevel: 'ERROR', sinceSeconds: 10800,
    });
    expect(screen.queryByText('Filters changed. Click Search to apply.')).not.toBeInTheDocument();
  });

  it('clears previous results and prevents duplicate submits while pending', async () => {
    const user = userEvent.setup();
    const next = deferred();
    mockedInvoke.mockResolvedValueOnce(matchedResponse).mockReturnValueOnce(next.promise);
    renderSearch();
    const button = screen.getByRole('button', { name: 'Search' });
    await user.click(button);
    await screen.findByRole('table');
    await user.click(button);
    expect(button).toBeDisabled();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Export CSV' })).not.toBeInTheDocument();
    await user.click(button);
    await user.type(screen.getByRole('textbox', { name: 'Keyword' }), '{Enter}');
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
    await act(async () => next.resolve(emptyResponse));
    await screen.findByText('No results found');
    expect(button).toBeEnabled();
  });

  it('shows global error and retries the previous snapshot without automatic retries', async () => {
    const user = userEvent.setup();
    mockedInvoke.mockRejectedValueOnce('Kubernetes error: access denied');
    renderSearch();
    await user.type(screen.getByRole('textbox', { name: 'Keyword' }), 'forecast sync');
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByText('Search failed');
    expect(screen.getByRole('alert')).toHaveTextContent('Kubernetes error: access denied');
    expect(screen.queryByText('No results found')).not.toBeInTheDocument();
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
    await user.clear(screen.getByRole('textbox', { name: 'Keyword' }));
    await user.type(screen.getByRole('textbox', { name: 'Keyword' }), 'changed');
    await user.click(screen.getByRole('button', { name: 'Retry' }));
    await screen.findByText('No results found');
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
    expect(mockedInvoke.mock.calls[1]).toEqual(mockedInvoke.mock.calls[0]);
    expect(screen.getByText('Filters changed. Click Search to apply.')).toBeInTheDocument();
  });

  it.each(['context', 'namespace', 'deployment'] as const)(
    'resets when %s changes and ignores the late response even after returning to the old scope',
    async (field) => {
      const user = userEvent.setup();
      const old = deferred();
      mockedInvoke.mockReturnValueOnce(old.promise);
      renderSearch();
      await user.click(screen.getByRole('button', { name: 'Search' }));
      const original = useClusterStore.getState()[field];
      act(() => useClusterStore.setState({ [field]: 'other' }));
      expect(screen.getByText('Search across all pods in a deployment')).toBeInTheDocument();
      act(() => useClusterStore.setState({ [field]: original }));
      await user.click(screen.getByRole('button', { name: 'Search' }));
      await screen.findByText('No results found');
      await act(async () => old.resolve(matchedResponse));
      expect(screen.getByText('No results found')).toBeInTheDocument();
      expect(screen.queryByRole('table')).not.toBeInTheDocument();
      expect(mockedInvoke).toHaveBeenCalledTimes(2);
    }
  );

  it('ignores late errors after a scope change', async () => {
    const user = userEvent.setup();
    const old = deferred();
    mockedInvoke.mockReturnValueOnce(old.promise);
    renderSearch();
    await user.click(screen.getByRole('button', { name: 'Search' }));
    act(() => useClusterStore.getState().setDeployment('other'));
    await act(async () => old.reject(new Error('Old scope failed')));
    expect(screen.getByText('Search across all pods in a deployment')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('shows a 30-second timeout as an error and ignores late success', async () => {
    vi.useFakeTimers();
    const old = deferred();
    mockedInvoke.mockReturnValueOnce(old.promise);
    renderSearch();
    fireEvent.click(screen.getByRole('button', { name: 'Search' }));
    await act(async () => vi.advanceTimersByTimeAsync(0));
    await act(async () => vi.advanceTimersByTimeAsync(30000));
    expect(screen.getByText('Search failed')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toHaveTextContent('Connection timed out after 30 seconds');
    expect(screen.queryByText('No results found')).not.toBeInTheDocument();
    await act(async () => old.resolve(matchedResponse));
    expect(screen.getByText('Search failed')).toBeInTheDocument();
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    expect(mockedInvoke).toHaveBeenCalledTimes(1);
  });
});

describe('search completeness', () => {
  it('shows matches alongside partial failure details', async () => {
    mockedInvoke.mockResolvedValue({
      ...matchedResponse, total_containers: 2, failures: [failure],
    });
    const user = userEvent.setup();
    renderSearch();
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByRole('table');
    expect(screen.getByRole('alert')).toHaveTextContent('Search incomplete: 1 of 2 containers searched successfully.');
    await user.click(screen.getByText('Failed containers (1)'));
    expect(screen.getByText('queue-b / sidecar')).toBeVisible();
    expect(screen.getByText(/Access denied/)).toBeVisible();
  });

  it('distinguishes partial search without matches from a complete empty result', async () => {
    mockedInvoke.mockResolvedValue({ ...emptyResponse, total_containers: 2, failures: [failure] });
    const user = userEvent.setup();
    renderSearch();
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByText('No matches in successfully searched containers');
    expect(screen.getByRole('alert')).toHaveTextContent('Search incomplete');
    expect(screen.queryByText('No results found')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInTheDocument();
  });

  it('shows all-container failures as error and allows Retry', async () => {
    mockedInvoke.mockResolvedValueOnce({
      results: [], failures: [failure], total_containers: 1, successful_containers: 0,
    });
    const user = userEvent.setup();
    renderSearch();
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByText('Search failed');
    expect(screen.getByRole('alert')).toHaveTextContent('Could not read logs from any container.');
    expect(screen.queryByText('No results found')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Retry' }));
    await screen.findByText('No results found');
    expect(mockedInvoke).toHaveBeenCalledTimes(2);
  });

  it('shows a specific message when the deployment has zero search targets', async () => {
    mockedInvoke.mockResolvedValue({ ...emptyResponse, total_containers: 0, successful_containers: 0 });
    const user = userEvent.setup();
    renderSearch();
    await user.click(screen.getByRole('button', { name: 'Search' }));
    await screen.findByText('No pods or containers found');
    expect(screen.queryByText('No results found')).not.toBeInTheDocument();
    expect(screen.queryByText('Search failed')).not.toBeInTheDocument();
  });
});
