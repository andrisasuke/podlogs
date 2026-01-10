import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export type Theme = 'light' | 'dark' | 'system';
export type View = 'pods' | 'deployments' | 'logs' | 'search';
export type DrawerType = 'pod' | 'deployment' | null;

interface UIState {
  theme: Theme;
  sidebarCollapsed: boolean;
  currentView: View;
  selectedPod: string | null;
  selectedDeploymentInfo: string | null;
  drawerOpen: boolean;
  drawerType: DrawerType;
  logViewerPod: string | null;

  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
  toggleSidebar: () => void;
  setView: (view: View) => void;
  selectPod: (pod: string | null) => void;
  selectDeploymentInfo: (deployment: string | null) => void;
  setDrawerOpen: (open: boolean) => void;
  openLogViewer: (podName: string) => void;
  closeLogViewer: () => void;
}

export const useUIStore = create<UIState>()(
  persist(
    (set, get) => ({
      theme: 'dark',
      sidebarCollapsed: false,
      currentView: 'deployments',
      selectedPod: null,
      selectedDeploymentInfo: null,
      drawerOpen: false,
      drawerType: null,
      logViewerPod: null,

      setTheme: (theme) => set({ theme }),
      toggleTheme: () => {
        const current = get().theme;
        const next = current === 'dark' ? 'light' : 'dark';
        set({ theme: next });
      },
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setView: (currentView) => set({ currentView, logViewerPod: null }),
      selectPod: (selectedPod) => set({ selectedPod, drawerOpen: !!selectedPod, drawerType: selectedPod ? 'pod' : null }),
      selectDeploymentInfo: (selectedDeploymentInfo) => set({ selectedDeploymentInfo, drawerOpen: !!selectedDeploymentInfo, drawerType: selectedDeploymentInfo ? 'deployment' : null }),
      setDrawerOpen: (drawerOpen) => set({ drawerOpen, drawerType: drawerOpen ? get().drawerType : null }),
      openLogViewer: (podName) => set({ currentView: 'logs', logViewerPod: podName }),
      closeLogViewer: () => set({ currentView: 'pods', logViewerPod: null }),
    }),
    {
      name: 'podlogs-ui',
      partialize: (state) => ({ theme: state.theme, sidebarCollapsed: state.sidebarCollapsed }),
    }
  )
);
