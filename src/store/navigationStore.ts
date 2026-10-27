import { create } from 'zustand';

export type Route = 
  | { type: 'library' }
  | { type: 'recent' }
  | { type: 'stats' }
  | { type: 'settings' }
  | { type: 'playing' }
  | { type: 'game_details'; gameId: number };

interface NavigationState {
  currentRoute: Route;
  history: Route[];
  navigate: (route: Route) => void;
  goBack: () => void;
}

export const useNavigationStore = create<NavigationState>((set, get) => ({
  currentRoute: { type: 'library' },
  history: [],
  navigate: (route) => {
    const current = get().currentRoute;
    // Don't push to history if it's the exact same route
    if (JSON.stringify(current) === JSON.stringify(route)) return;
    
    set((state) => ({
      currentRoute: route,
      history: [...state.history, current],
    }));
  },
  goBack: () => {
    set((state) => {
      if (state.history.length === 0) return { currentRoute: { type: 'library' } };
      
      const newHistory = [...state.history];
      const previousRoute = newHistory.pop()!;
      
      return {
        currentRoute: previousRoute,
        history: newHistory,
      };
    });
  },
}));