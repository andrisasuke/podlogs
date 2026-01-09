import { use } from 'react';
import { ThemeContext } from '../contexts/ThemeContext';
import { useUIStore } from '../stores/uiStore';

/**
 * Hook to access theme state
 * Uses React 19's `use` hook for context reading
 */
export function useTheme() {
  // React 19's `use` hook - can be called conditionally unlike useContext
  const themeContext = use(ThemeContext);
  const { setTheme } = useUIStore();

  if (!themeContext) {
    throw new Error('useTheme must be used within ThemeProvider');
  }

  return {
    theme: themeContext.theme,
    isDark: themeContext.isDark,
    setTheme,
    toggleTheme: themeContext.toggleTheme,
  };
}
