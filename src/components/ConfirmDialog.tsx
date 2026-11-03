import { useState } from 'react';
import clsx from 'clsx';
import { Modal } from './Modal';
import { useConfirmStore } from '../store/confirmStore';

/** Renders the confirmation requested through confirmAction / confirmWithOption. */
export function ConfirmDialog() {
  const request = useConfirmStore((s) => s.request);
  if (!request) return null;
  return <ConfirmPanel key={request.title + request.message} />;
}

function ConfirmPanel() {
  const { request, close } = useConfirmStore();
  const [checked, setChecked] = useState(false);
  if (!request) return null;

  const cancel = () => close({ confirmed: false, checked: false });

  return (
    <Modal title={request.title} onClose={cancel}>
      <p className="text-gray-300 whitespace-pre-line">{request.message}</p>
      {request.checkboxLabel && (
        <label className="flex items-center gap-3 mt-5 text-sm text-gray-300 cursor-pointer">
          <input
            type="checkbox"
            checked={checked}
            onChange={(e) => setChecked(e.target.checked)}
            className="w-4 h-4 accent-red-500"
          />
          {request.checkboxLabel}
        </label>
      )}
      <div className="flex justify-end gap-3 mt-8">
        <button
          onClick={cancel}
          className="px-5 py-2.5 rounded-lg bg-gray-800 hover:bg-gray-700 border border-gray-700 text-white text-sm font-medium"
        >
          Cancel
        </button>
        <button
          onClick={() => close({ confirmed: true, checked })}
          className={clsx(
            'px-5 py-2.5 rounded-lg text-white text-sm font-medium',
            request.danger ? 'bg-red-600 hover:bg-red-700' : 'bg-indigo-600 hover:bg-indigo-500',
          )}
        >
          {request.confirmLabel}
        </button>
      </div>
    </Modal>
  );
}
