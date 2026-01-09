import { create } from 'zustand';

interface ErrorState {
  error: Error | null;
  isTransitioning: boolean;
  setError: (error: Error | null) => void;
  clearError: () => void;
  startTransition: () => void;
  endTransition: () => void;
}

export const useErrorStore = create<ErrorState>((set, get) => ({
  error: null,
  isTransitioning: false,
  setError: (error) => {
    // Don't set errors during namespace/context transitions
    if (!get().isTransitioning) {
      set({ error });
    }
  },
  clearError: () => set({ error: null }),
  startTransition: () => set({ isTransitioning: true, error: null }),
  endTransition: () => set({ isTransitioning: false }),
}));
