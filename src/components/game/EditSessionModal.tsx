import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Modal } from '../Modal';
import { GameSessionDto } from '../../types';
import { formatPlaytime, toDateTimeLocal } from '../../utils/formatters';
import { describeError } from '../../utils/errors';
import { toast } from '../../store/toastStore';

interface EditSessionModalProps {
  session: GameSessionDto;
  onClose: () => void;
}

export function EditSessionModal({ session, onClose }: EditSessionModalProps) {
  const [start, setStart] = useState(toDateTimeLocal(session.started_at));
  const [end, setEnd] = useState(toDateTimeLocal(session.ended_at ?? session.started_at));
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);

  const startDate = new Date(start);
  const endDate = new Date(end);
  const duration = Math.floor((endDate.getTime() - startDate.getTime()) / 1000);

  const save = async (e: React.FormEvent) => {
    e.preventDefault();
    setSaving(true);
    setError('');
    try {
      await invoke('update_session', {
        sessionId: session.session_id,
        startedAt: startDate.toISOString(),
        endedAt: endDate.toISOString(),
      });
      toast.success('Session updated.');
      onClose();
    } catch (err) {
      setError(describeError(err));
      setSaving(false);
    }
  };

  const inputClass = 'w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-indigo-500 [color-scheme:dark]';

  return (
    <Modal title="Edit Session" onClose={onClose} locked={saving}>
      <form onSubmit={save} className="space-y-4">
        {error && <div className="p-3 bg-red-500/10 text-red-400 border border-red-500/50 rounded" role="alert">{error}</div>}
        <div>
          <label htmlFor="session-start" className="block text-sm font-medium text-gray-400 mb-1">Started</label>
          <input id="session-start" type="datetime-local" value={start} onChange={(e) => setStart(e.target.value)} className={inputClass} required />
        </div>
        <div>
          <label htmlFor="session-end" className="block text-sm font-medium text-gray-400 mb-1">Ended</label>
          <input id="session-end" type="datetime-local" value={end} onChange={(e) => setEnd(e.target.value)} className={inputClass} required />
        </div>
        <p className="text-sm text-gray-400">
          Duration: <span className="text-white font-medium">{duration > 0 ? formatPlaytime(duration) : '—'}</span>
        </p>
        <div className="flex justify-end gap-3 pt-4">
          <button type="button" onClick={onClose} className="px-5 py-2.5 rounded-lg bg-gray-800 hover:bg-gray-700 border border-gray-700 text-sm font-medium">
            Cancel
          </button>
          <button type="submit" disabled={saving || duration <= 0} className="px-5 py-2.5 rounded-lg bg-indigo-600 hover:bg-indigo-500 text-sm font-medium disabled:opacity-50">
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </form>
    </Modal>
  );
}
