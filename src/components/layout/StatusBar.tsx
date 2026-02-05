import { useState, useEffect, useRef } from 'react';
import { useClusterStore } from '../../stores/clusterStore';
import { usePods } from '../../hooks/useK8s';
import { useUIStore } from '../../stores/uiStore';
import { STATUS_BAR_HEIGHT } from '../../lib/constants';

export function StatusBar() {
  const { context, deployment } = useClusterStore();
  const { currentView, refreshInterval } = useUIStore();
  const { data: pods = [], isFetching } = usePods(deployment ?? undefined);
  const [countdown, setCountdown] = useState(Math.floor(refreshInterval / 1000));
  const wasFetchingRef = useRef(false);

  // Reset countdown when fetch completes or refreshInterval changes
  useEffect(() => {
    if (wasFetchingRef.current && !isFetching) {
      setCountdown(Math.floor(refreshInterval / 1000));
    }
    wasFetchingRef.current = isFetching;
  }, [isFetching, refreshInterval]);

  // Reset countdown when refreshInterval changes
  useEffect(() => {
    setCountdown(Math.floor(refreshInterval / 1000));
  }, [refreshInterval]);

  // Countdown timer
  useEffect(() => {
    const timer = window.setInterval(() => {
      setCountdown((prev) => (prev > 0 ? prev - 1 : 0));
    }, 1000);

    return () => window.clearInterval(timer);
  }, []);

  return (
    <div
      className="flex items-center justify-between px-4 text-xs text-text-muted border-t border-border bg-bg-secondary"
      style={{ height: STATUS_BAR_HEIGHT }}
    >
      <div className="flex items-center gap-3">
        {context && (
          <>
            <div className="flex items-center gap-1.5">
              {isFetching ? (
                <span className="w-2 h-2 rounded-full bg-amber-500 animate-pulse" />
              ) : (
                <span className="w-2 h-2 rounded-full bg-emerald-500" />
              )}
              <span>{isFetching ? 'Syncing...' : 'Live'}</span>
            </div>
          </>
        )}
      </div>

      <div className="flex items-center gap-4">
        {currentView === 'pods' && (
          <span>
            Showing {pods.length} pod{pods.length !== 1 ? 's' : ''}
          </span>
        )}
        {!isFetching && (
          <span>Refresh in {countdown}s</span>
        )}
      </div>
    </div>
  );
}
