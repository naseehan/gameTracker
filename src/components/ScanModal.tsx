import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { GameCandidate } from '../types';
import { useLibraryStore } from '../store/libraryStore';
import { Check, FolderSearch, ChevronRight } from 'lucide-react';
import { Modal } from './Modal';
import { toast } from '../store/toastStore';
import { describeError, reportError } from '../utils/errors';

/** Matches the backend's name normalization, to spot games already in the library. */
export function normalizeName(name: string): string {
  return name.toLowerCase().replace(/[^\p{L}\p{N}]/gu, '');
}

interface ScanModalProps {
  onClose: () => void;
}

export function ScanModal({ onClose }: ScanModalProps) {
  const [isScanning, setIsScanning] = useState(false);
  const [candidates, setCandidates] = useState<GameCandidate[] | null>(null);
  const [selectedIndices, setSelectedIndices] = useState<Set<number>>(new Set());
  const [isAdding, setIsAdding] = useState(false);
  const games = useLibraryStore((s) => s.games);
  const inLibrary = new Set(games.map((g) => normalizeName(g.name)));

  const handleScanFolder = async () => {
    try {
      setIsScanning(true);
      const results = await invoke<GameCandidate[] | null>('pick_and_scan_folder');
      if (!results) return;
      setCandidates(results);
      // Preselect the games that aren't in the library yet.
      setSelectedIndices(new Set(results.flatMap((c, i) => (inLibrary.has(normalizeName(c.name)) ? [] : [i]))));
    } catch (err) {
      reportError("Couldn't scan the folder", err);
    } finally {
      setIsScanning(false);
    }
  };

  const handleAddSelected = async () => {
    if (!candidates) return;
    setIsAdding(true);
    const failed: string[] = [];
    for (const idx of selectedIndices) {
      try {
        await invoke('add_game', { candidate: candidates[idx] });
      } catch (err) {
        failed.push(`${candidates[idx].name} (${describeError(err)})`);
      }
    }
    setIsAdding(false);
    if (failed.length > 0) {
      reportError("Some games couldn't be added", failed.join(', '));
    } else {
      toast.success(`Added ${selectedIndices.size} game${selectedIndices.size === 1 ? '' : 's'}.`);
      onClose();
    }
  };

  const toggleSelection = (index: number) => {
    const newSet = new Set(selectedIndices);
    if (newSet.has(index)) {
      newSet.delete(index);
    } else {
      newSet.add(index);
    }
    setSelectedIndices(newSet);
  };

  return (
    <Modal title={candidates ? 'Games Found' : 'Scan a Folder'} onClose={onClose} width="max-w-xl" locked={isAdding}>
      {!candidates && (
        <div className="flex flex-col items-center text-center">
          <p className="text-gray-400 mb-8 max-w-md">
            Choose a game's folder, or a folder with several games in it such as D:\Games. Each game's programs are found automatically.
          </p>
          <button
            onClick={handleScanFolder}
            disabled={isScanning}
            className="w-full flex items-center justify-between p-5 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-xl transition-all group disabled:opacity-50"
          >
            <div className="flex items-center space-x-4">
              <FolderSearch className="text-indigo-400 group-hover:scale-110 transition-transform" />
              <span className="font-medium">{isScanning ? 'Scanning...' : 'Choose Folder to Scan'}</span>
            </div>
            {!isScanning && <ChevronRight className="text-gray-500" />}
          </button>
        </div>
      )}

      {candidates && (
        <div className="flex flex-col h-[400px]">
          <div className="flex justify-between items-center mb-4 px-1">
            <button
              onClick={() => setSelectedIndices(selectedIndices.size === candidates.length ? new Set() : new Set(candidates.map((_, i) => i)))}
              className="text-indigo-400 hover:text-indigo-300 text-sm font-medium"
            >
              {selectedIndices.size === candidates.length ? 'Clear All' : 'Select All'}
            </button>
            <span className="text-sm text-gray-500">{selectedIndices.size} selected</span>
          </div>

          <div className="flex-1 overflow-y-auto space-y-2 pr-2">
            {candidates.length === 0 ? (
              <div className="h-full flex items-center justify-center text-gray-500 text-center">
                No games found in that folder.
              </div>
            ) : (
              candidates.map((cand, idx) => (
                <label
                  key={cand.install_path}
                  className={`flex items-start p-4 rounded-xl border cursor-pointer transition-all ${selectedIndices.has(idx) ? 'bg-indigo-900/20 border-indigo-500/50' : 'bg-gray-800/50 border-gray-800 hover:border-gray-700'}`}
                >
                  <input type="checkbox" className="sr-only" checked={selectedIndices.has(idx)} onChange={() => toggleSelection(idx)} />
                  <div className={`mt-1 mr-4 rounded border ${selectedIndices.has(idx) ? 'bg-indigo-500 border-indigo-500' : 'bg-gray-900 border-gray-600'} w-5 h-5 flex items-center justify-center flex-shrink-0`} aria-hidden="true">
                    {selectedIndices.has(idx) && <Check size={14} className="text-white" />}
                  </div>
                  <div className="flex-1 overflow-hidden">
                    <h3 className="font-bold text-gray-200">
                      {cand.name}
                      {inLibrary.has(normalizeName(cand.name)) && (
                        <span className="ml-2 text-xs font-medium text-gray-500">Already in library</span>
                      )}
                    </h3>
                    <p className="text-xs text-gray-500 truncate" title={cand.install_path}>{cand.install_path}</p>
                    <p className="text-xs text-gray-600 mt-0.5">
                      {cand.executables.length} program{cand.executables.length === 1 ? '' : 's'} found
                    </p>
                  </div>
                </label>
              ))
            )}
          </div>

          <div className="mt-6 flex justify-between">
            <button onClick={() => setCandidates(null)} className="px-4 py-2.5 text-gray-400 hover:text-white text-sm font-medium">
              Scan another folder
            </button>
            <button
              onClick={handleAddSelected}
              disabled={isAdding || selectedIndices.size === 0}
              className="px-6 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white font-medium rounded-lg transition-all disabled:opacity-50 flex items-center"
            >
              {isAdding ? 'Adding...' : 'Add Selected Games'}
            </button>
          </div>
        </div>
      )}
    </Modal>
  );
}
