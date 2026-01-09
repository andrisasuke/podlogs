import { createContext, use, type ReactNode } from 'react';

export type Theme = 'light' | 'dark';

interface ThemeContextValue {
  theme: Theme;
  isDark: boolean;
  toggleTheme: () => void;
}

export const ThemeContext = createContext<ThemeContextValue | null>(null);

/**
 * Custom hook using React 19's `use` API
 * Unlike useContext, `use` can be called conditionally (inside if/loops)
 */
export function useThemeContext(options?: { optional?: boolean }) {
  const context = use(ThemeContext);

  if (!context && !options?.optional) {
    throw new Error('useThemeContext must be used within a ThemeProvider');
  }

  return context;
}

/**
 * Example of conditional context reading with `use`
 * This is NOT possible with useContext!
 */
export function useThemeIfAvailable(): ThemeContextValue | null {
  // With React 19's `use`, we can read context conditionally
  // This would NOT work with useContext (must be at top level)
  try {
    return use(ThemeContext);
  } catch {
    return null;
  }
}

interface ThemeProviderProps {
  children: ReactNode;
  value: ThemeContextValue;
}

export function ThemeProvider({ children, value }: ThemeProviderProps) {
  return (
    <ThemeContext.Provider value={value}>
      {children}
    </ThemeContext.Provider>
  );
}
