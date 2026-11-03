import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { FileCode, Folder, Plus, X } from 'lucide-react';
import { DetectionEntry, GameDetection } from '../../types';
import { reportError } from '../../utils/errors';
import { useBackendEvent } from '../../hooks/useBackendEvent';

/** Shows and edits what identifies a game's processes: its programs and install folders. */
export function DetectionCard({ gameId }: { gameId: number }) {
  const [detection, setDetection] = useState<GameDetection | null>(null);

  const load = () => {
    invoke<GameDetection>('get_game_detection', { gameId })
      .then(setDetection)
      .catch((err) => reportError("Couldn't load detection settings", err));
  };
  useEffect(load, [gameId]);
  useBackendEvent(['library-changed'], load);

  const run = (command: string, args: Record<string, unknown>, failure: string) => {
    invoke(command, args).catch((err) => reportError(failure, err));
  };

  if (!detection) return null;
  const nothing = detection.executables.length === 0 && detection.install_folders.length === 0;

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-6 shadow-lg">
      <h2 className="text-lg font-bold text-white mb-1">Detection</h2>
      <p className="text-sm text-gray-400 mb-5">
        Any program started from an install folder counts as this game, as do the programs listed here.
      </p>
      {nothing && (
        <p className="text-sm text-amber-300 bg-amber-500/10 border border-amber-500/30 rounded-lg p-3 mb-5">
          Nothing identifies this game yet, so it won't be tracked. Add its install folder or program.
        </p>
      )}

      <EntryList
        title="Install folders"
        icon={<Folder size={14} />}
        entries={detection.install_folders}
        addLabel="Add folder"
        onAdd={() => run('add_game_install_folder', { gameId }, "Couldn't add the folder")}
        onRemove={(id) => run('remove_game_install_folder', { installId: id }, "Couldn't remove the folder")}
      />
      <div className="h-5" />
      <EntryList
        title="Programs"
        icon={<FileCode size={14} />}
        entries={detection.executables}
        addLabel="Add program"
        onAdd={() => run('add_game_executable', { gameId }, "Couldn't add the program")}
        onRemove={(id) => run('remove_game_executable', { executableId: id }, "Couldn't remove the program")}
      />
    </div>
  );
}

interface EntryListProps {
  title: string;
  icon: React.ReactNode;
  entries: DetectionEntry[];
  addLabel: string;
  onAdd: () => void;
  onRemove: (id: number) => void;
}

function EntryList({ title, icon, entries, addLabel, onAdd, onRemove }: EntryListProps) {
  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-gray-500">{title}</h3>
        <button onClick={onAdd} className="flex items-center gap-1 text-sm font-medium text-indigo-400 hover:text-indigo-300">
          <Plus size={14} /> {addLabel}
        </button>
      </div>
      {entries.length === 0 ? (
        <p className="text-sm text-gray-600">None</p>
      ) : (
        <ul className="space-y-1.5">
          {entries.map((entry) => (
            <li key={entry.id} className="flex items-center gap-2 text-sm text-gray-300 font-mono bg-gray-800/50 px-3 py-2 rounded-lg border border-gray-700/50">
              <span className="text-gray-500 flex-shrink-0">{icon}</span>
              <span className="flex-1 break-all">{entry.path}</span>
              <button onClick={() => onRemove(entry.id)} className="p-1 text-gray-500 hover:text-red-300 rounded" aria-label={`Remove ${entry.path}`}>
                <X size={14} />
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
