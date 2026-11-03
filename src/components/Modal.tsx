import { ReactNode, useEffect, useRef } from 'react';
import { X } from 'lucide-react';

interface ModalProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  /** Tailwind max-width class for the panel. */
  width?: string;
  /** Set while work is in progress, so the modal can't be dismissed halfway through. */
  locked?: boolean;
}

/** A dialog that closes on Escape or a click outside, and keeps keyboard focus inside it. */
export function Modal({ title, onClose, children, width = 'max-w-md', locked = false }: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const focusable = panelRef.current?.querySelector<HTMLElement>(
      'input, select, textarea, button:not([aria-label="Close"])',
    );
    (focusable ?? panelRef.current)?.focus();
    return () => previous?.focus();
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !locked) onClose();
      if (e.key !== 'Tab' || !panelRef.current) return;
      const items = panelRef.current.querySelectorAll<HTMLElement>(
        'button:not([disabled]), input:not([disabled]), select, textarea, [tabindex="0"]',
      );
      if (items.length === 0) return;
      const first = items[0];
      const last = items[items.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [onClose, locked]);

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
      onClick={locked ? undefined : onClose}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        className={`bg-gray-900 border border-gray-800 rounded-xl p-8 w-full ${width} max-h-[90vh] overflow-y-auto shadow-2xl relative focus:outline-none`}
        onClick={(e) => e.stopPropagation()}
      >
        {!locked && (
          <button
            className="absolute top-4 right-4 text-gray-500 hover:text-white"
            onClick={onClose}
            aria-label="Close"
          >
            <X size={20} />
          </button>
        )}
        <h2 className="text-2xl font-bold mb-6 pr-8">{title}</h2>
        {children}
      </div>
    </div>
  );
}
