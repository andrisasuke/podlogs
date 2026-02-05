import { Dropdown } from '../common/Dropdown';
import { useUIStore, Theme, RefreshInterval } from '../../stores/uiStore';

const themeOptions = [
  { value: 'dark', label: 'Dark' },
  { value: 'light', label: 'Light' },
  { value: 'system', label: 'System' },
];

const refreshIntervalOptions = [
  { value: '5000', label: '5 seconds' },
  { value: '10000', label: '10 seconds' },
  { value: '15000', label: '15 seconds' },
  { value: '20000', label: '20 seconds' },
  { value: '30000', label: '30 seconds' },
];

export function Settings() {
  const { theme, setTheme, refreshInterval, setRefreshInterval } = useUIStore();

  return (
    <div className="p-4 space-y-6">
      <div>
        <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
          Theme
        </label>
        <Dropdown
          options={themeOptions}
          value={theme}
          onChange={(value) => setTheme(value as Theme)}
        />
        <p className="text-xs text-text-muted mt-2">
          Choose your preferred color theme
        </p>
      </div>

      <div>
        <label className="block text-xs font-medium text-text-muted uppercase tracking-wider mb-2">
          Refresh Interval
        </label>
        <Dropdown
          options={refreshIntervalOptions}
          value={String(refreshInterval)}
          onChange={(value) => setRefreshInterval(Number(value) as RefreshInterval)}
        />
        <p className="text-xs text-text-muted mt-2">
          How often to refresh logs and pod status
        </p>
      </div>
    </div>
  );
}
