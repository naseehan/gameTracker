import { create } from 'zustand';

export interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel: string;
  danger?: boolean;
  /** An optional checkbox, e.g. "Also delete play history". */
  checkboxLabel?: string;
}

export interface ConfirmResult {
  confirmed: boolean;
  checked: boolean;
}

interface ConfirmState {
  request: (ConfirmOptions & { resolve: (result: ConfirmResult) => void }) | null;
  open: (options: ConfirmOptions) => Promise<ConfirmResult>;
  close: (result: ConfirmResult) => void;
}

export const useConfirmStore = create<ConfirmState>((set, get) => ({
  request: null,
  open: (options) =>
    new Promise((resolve) => {
      get().request?.resolve({ confirmed: false, checked: false });
      set({ request: { ...options, resolve } });
    }),
  close: (result) => {
    get().request?.resolve(result);
    set({ request: null });
  },
}));

/** Asks the user to confirm an action. Resolves to true if they did. */
export async function confirmAction(options: ConfirmOptions): Promise<boolean> {
  return (await useConfirmStore.getState().open(options)).confirmed;
}

/** Like confirmAction, with a checkbox whose state is returned too. */
export function confirmWithOption(options: ConfirmOptions & { checkboxLabel: string }) {
  return useConfirmStore.getState().open(options);
}
