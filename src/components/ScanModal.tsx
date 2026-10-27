import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { GameCandidate } from '../types';
import { useLibraryStore } from '../store/libraryStore';
import { Check, FolderSearch, Search, ChevronRight } from 'lucide-react';

interface ScanModalProps {
  onClose: () => void;
}

export function ScanModal({ onClose }: ScanModalProps) {
  const [step, setStep] = useState(1);
  const [isScanning, setIsScanning] = useState(false);
  const [candidates, setCandidates] = useState<GameCandidate[]>([]);
  const [selectedIndices, setSelectedIndices] = useState<Set<number>>(new Set());
  const [isAdding, setIsAdding] = useState(false);
  const fetchGames = useLibraryStore(s => s.fetchGames);

  const handleScanFolder = async () => {
    try {
      const selectedPath = await open({
        directory: true,
        multiple: false,
        title: 'Select Folder to Scan',
      });
      
      if (selectedPath && typeof selectedPath === 'string') {
        setIsScanning(true);
        const results = await invoke<GameCandidate[]>('scan_folder', { path: selectedPath });
        setCandidates(results);
        setSelectedIndices(new Set(results.map((_, i) => i)));
        setStep(2);
      }
    } catch (err) {
      console.error('Failed to scan folder', err);
    } finally {
      setIsScanning(false);
    }
  };

  const handleAddSelected = async () => {
    setIsAdding(true);
    try {
      for (const idx of selectedIndices) {
        await invoke('add_game', { candidate: candidates[idx] });
      }
      fetchGames();
      onClose();
    } catch (err) {
      console.error('Failed to add games', err);
    } finally {
      setIsAdding(false);
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
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-gray-900 border border-gray-800 rounded-xl p-8 max-w-xl w-full shadow-2xl relative flex flex-col" onClick={e => e.stopPropagation()}>
        <button className="absolute top-4 right-4 text-gray-500 hover:text-white" onClick={onClose}>✕</button>
        
        {step === 1 && (
          <div className="flex flex-col items-center text-center py-6">
            <div className="w-16 h-16 bg-gray-800 rounded-2xl flex items-center justify-center mb-8 shadow-inner">
              <Search size={32} className="text-indigo-400" />
            </div>
            <h2 className="text-2xl font-bold mb-4">Scan for Games</h2>
            <p className="text-gray-400 mb-8 max-w-md">Choose a folder to scan. We'll automatically identify games and their executables.</p>
            
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

        {step === 2 && (
          <div className="flex flex-col h-[400px]">
            <h2 className="text-2xl font-bold mb-2">Games Found</h2>
            
            <div className="flex justify-between items-center mb-4 px-2">
              <button 
                onClick={() => setSelectedIndices(selectedIndices.size === candidates.length ? new Set() : new Set(candidates.map((_, i) => i)))}
                className="text-indigo-400 hover:text-indigo-300 text-sm font-medium"
              >
                {selectedIndices.size === candidates.length ? 'Clear All' : 'Select All'}
              </button>
              <span className="text-sm text-gray-500">{selectedIndices.size} selected</span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2 pr-2 custom-scrollbar">
              {candidates.length === 0 ? (
                <div className="h-full flex items-center justify-center text-gray-500">
                  No games found in the selected folder.
                </div>
              ) : (
                candidates.map((cand, idx) => (
                  <div 
                    key={idx}
                    onClick={() => toggleSelection(idx)}
                    className={`flex items-start p-4 rounded-xl border cursor-pointer transition-all ${selectedIndices.has(idx) ? 'bg-indigo-900/20 border-indigo-500/50' : 'bg-gray-800/50 border-gray-800 hover:border-gray-700'}`}
                  >
                    <div className={`mt-1 mr-4 rounded border ${selectedIndices.has(idx) ? 'bg-indigo-500 border-indigo-500' : 'bg-gray-900 border-gray-600'} w-5 h-5 flex items-center justify-center`}>
                      {selectedIndices.has(idx) && <Check size={14} className="text-white" />}
                    </div>
                    <div className="flex-1 overflow-hidden">
                      <h3 className="font-bold text-gray-200">{cand.name}</h3>
                      <p className="text-xs text-gray-500 truncate" title={cand.install_path}>{cand.install_path}</p>
                    </div>
                  </div>
                ))
              )}
            </div>

            <div className="mt-6 flex justify-end">
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
      </div>
    </div>
  );
}
