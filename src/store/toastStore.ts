import { create } from 'zustand';

export type ToastKind = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  action?: { label: string; onClick: () => void };
}

interface ToastState {
  toasts: Toast[];
  show: (toast: Omit<Toast, 'id'>) => void;
  dismiss: (id: number) => void;
}

let nextId = 1;

export const useToastStore = create<ToastState>((set, get) => ({
  toasts: [],
  show: (toast) => {
    const id = nextId++;
    set({ toasts: [...get().toasts, { ...toast, id }] });
    // Errors and toasts with an action stay until dismissed or clicked.
    if (toast.kind !== 'error' && !toast.action) {
      setTimeout(() => get().dismiss(id), 4000);
    }
  },
  dismiss: (id) => set({ toasts: get().toasts.filter((t) => t.id !== id) }),
}));

export const toast = {
  success: (message: string) => useToastStore.getState().show({ kind: 'success', message }),
  info: (message: string, action?: Toast['action']) =>
    useToastStore.getState().show({ kind: 'info', message, action }),
  error: (message: string) => useToastStore.getState().show({ kind: 'error', message }),
};
