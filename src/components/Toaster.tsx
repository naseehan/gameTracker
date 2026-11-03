import { CheckCircle2, AlertCircle, Info, X } from 'lucide-react';
import clsx from 'clsx';
import { useToastStore } from '../store/toastStore';

const icons = {
  success: <CheckCircle2 size={18} className="text-green-400 flex-shrink-0" />,
  error: <AlertCircle size={18} className="text-red-400 flex-shrink-0" />,
  info: <Info size={18} className="text-indigo-400 flex-shrink-0" />,
};

export function Toaster() {
  const { toasts, dismiss } = useToastStore();

  return (
    <div className="fixed bottom-4 right-4 z-[60] flex flex-col gap-2 w-96 max-w-[calc(100vw-2rem)]" aria-live="polite">
      {toasts.map((t) => (
        <div
          key={t.id}
          role={t.kind === 'error' ? 'alert' : 'status'}
          className={clsx(
            'flex items-start gap-3 p-4 rounded-xl border shadow-2xl bg-gray-900',
            t.kind === 'error' ? 'border-red-900/60' : 'border-gray-700',
          )}
        >
          {icons[t.kind]}
          <p className="text-sm text-gray-200 flex-1 break-words">{t.message}</p>
          {t.action && (
            <button
              onClick={() => {
                t.action!.onClick();
                dismiss(t.id);
              }}
              className="text-sm font-medium text-indigo-400 hover:text-indigo-300"
            >
              {t.action.label}
            </button>
          )}
          <button onClick={() => dismiss(t.id)} aria-label="Dismiss" className="text-gray-500 hover:text-gray-300">
            <X size={16} />
          </button>
        </div>
      ))}
    </div>
  );
}
