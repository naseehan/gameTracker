import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GameCandidate } from '../types';
import { Modal } from './Modal';
import { toast } from '../store/toastStore';
import { describeError } from '../utils/errors';

interface AddGameModalProps {
  onClose: () => void;
}

/** The folder an executable is in, and a readable name from that folder. */
function folderAndName(exePath: string): { folder: string; name: string } {
  const parts = exePath.split(/[\\/]/);
  parts.pop();
  const folder = parts.join('\\');
  return { folder, name: parts[parts.length - 1] ?? '' };
}

export function AddGameModal({ onClose }: AddGameModalProps) {
  const [name, setName] = useState('');
  const [folder, setFolder] = useState('');
  const [executable, setExecutable] = useState('');
  const [isAdding, setIsAdding] = useState(false);
  const [error, setError] = useState('');

  const handleSelectFolder = async () => {
    try {
      const selected = await invoke<string | null>('pick_folder');
      if (selected) setFolder(selected);
    } catch (err) {
      setError(describeError(err));
    }
  };

  const handleSelectExe = async () => {
    try {
      const selected = await invoke<string | null>('pick_executable');
      if (!selected) return;
      setExecutable(selected);
      // Fill in the rest from the executable's location, unless already typed.
      const guess = folderAndName(selected);
      if (!folder) setFolder(guess.folder);
      if (!name) setName(guess.name);
    } catch (err) {
      setError(describeError(err));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim() || !executable.trim()) {
      setError('Choose the game\'s program and give it a name.');
      return;
    }

    setIsAdding(true);
    setError('');

    const candidate: GameCandidate = {
      name: name.trim(),
      install_path: folder.trim() || folderAndName(executable).folder,
      executables: [executable.trim()],
      source: 'Manual',
      confidence: 100
    };

    try {
      await invoke('add_game', { candidate });
      toast.success(`Added ${candidate.name}.`);
      onClose();
    } catch (err) {
      setError(describeError(err));
      setIsAdding(false);
    }
  };

  const inputClass = "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-indigo-500";
  const browseClass = "px-4 py-2 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-lg text-sm flex-shrink-0";

  return (
    <Modal title="Add Game" onClose={onClose} locked={isAdding}>
      {error && <div className="mb-4 p-3 bg-red-500/10 text-red-400 border border-red-500/50 rounded" role="alert">{error}</div>}

      <form onSubmit={handleSubmit} className="space-y-4">
        <div>
          <label htmlFor="add-exe" className="block text-sm font-medium text-gray-400 mb-1">Game program (.exe)</label>
          <div className="flex space-x-2">
            <input
              id="add-exe"
              type="text"
              value={executable}
              onChange={e => setExecutable(e.target.value)}
              className={inputClass}
              placeholder="D:\Games\Hades\Hades.exe"
            />
            <button type="button" onClick={handleSelectExe} className={browseClass}>Browse</button>
          </div>
        </div>

        <div>
          <label htmlFor="add-name" className="block text-sm font-medium text-gray-400 mb-1">Name</label>
          <input
            id="add-name"
            type="text"
            value={name}
            onChange={e => setName(e.target.value)}
            className={inputClass}
            placeholder="e.g. Hades"
          />
        </div>

        <div>
          <label htmlFor="add-folder" className="block text-sm font-medium text-gray-400 mb-1">
            Install folder <span className="text-gray-600">(anything run from here counts as this game)</span>
          </label>
          <div className="flex space-x-2">
            <input
              id="add-folder"
              type="text"
              value={folder}
              onChange={e => setFolder(e.target.value)}
              className={inputClass}
              placeholder="D:\Games\Hades"
            />
            <button type="button" onClick={handleSelectFolder} className={browseClass}>Browse</button>
          </div>
        </div>

        <button
          type="submit"
          disabled={isAdding}
          className="w-full mt-6 py-3 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-lg transition-colors disabled:opacity-50"
        >
          {isAdding ? 'Adding...' : 'Add Game'}
        </button>
      </form>
    </Modal>
  );
}
