import { useEffect, useMemo } from 'react';
import { MainLayout } from './components/layout/MainLayout';
import { useUIStore } from './stores/uiStore';
import { ThemeProvider, type Theme } from './contexts/ThemeContext';

function App() {
  const { theme, toggleTheme } = useUIStore();

  const isDark = useMemo(() =>
    theme === 'dark' || (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches),
    [theme]
  );

  useEffect(() => {
    const root = document.documentElement;
    root.classList.toggle('dark', isDark);
  }, [isDark]);

  useEffect(() => {
    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const handler = (e: MediaQueryListEvent) => {
        document.documentElement.classList.toggle('dark', e.matches);
      };
      mediaQuery.addEventListener('change', handler);
      return () => mediaQuery.removeEventListener('change', handler);
    }
  }, [theme]);

  // Provide theme context using React 19's Context + use() pattern
  const themeContextValue = useMemo(() => ({
    theme: isDark ? 'dark' : 'light' as Theme,
    isDark,
    toggleTheme,
  }), [isDark, toggleTheme]);

  return (
    <ThemeProvider value={themeContextValue}>
      <MainLayout />
    </ThemeProvider>
  );
}

export default App;
