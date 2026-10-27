import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { GameCandidate } from '../types';
import { useLibraryStore } from '../store/libraryStore';

interface AddGameModalProps {
  onClose: () => void;
}

export function AddGameModal({ onClose }: AddGameModalProps) {
  const [name, setName] = useState('');
  const [folder, setFolder] = useState('');
  const [executable, setExecutable] = useState('');
  const [isAdding, setIsAdding] = useState(false);
  const [error, setError] = useState('');
  const fetchGames = useLibraryStore(s => s.fetchGames);

  const handleSelectFolder = async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === 'string') {
        setFolder(selected);
      }
    } catch (err) {
      console.error(err);
    }
  };

  const handleSelectExe = async () => {
    try {
      const selected = await open({ 
        directory: false, 
        multiple: false,
        filters: [{ name: 'Executables', extensions: ['exe'] }]
      });
      if (selected && typeof selected === 'string') {
        setExecutable(selected);
      }
    } catch (err) {
      console.error(err);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name || !folder || !executable) {
      setError('Please fill all fields');
      return;
    }
    
    setIsAdding(true);
    setError('');

    const candidate: GameCandidate = {
      name,
      install_path: folder,
      executables: [executable],
      source: 'Manual',
      confidence: 100
    };

    try {
      await invoke('add_game', { candidate });
      fetchGames(); // Refresh library
      onClose();
    } catch (err: any) {
      setError(err.toString());
      setIsAdding(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={onClose}>
      <div className="bg-gray-900 border border-gray-800 rounded-xl p-8 max-w-md w-full shadow-2xl relative" onClick={e => e.stopPropagation()}>
        <button className="absolute top-4 right-4 text-gray-500 hover:text-white" onClick={onClose}>✕</button>
        <h2 className="text-2xl font-bold mb-6">Add Game</h2>
        
        {error && <div className="mb-4 p-3 bg-red-500/10 text-red-400 border border-red-500/50 rounded">{error}</div>}
        
        <form onSubmit={handleSubmit} className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">Game Name</label>
            <input 
              type="text" 
              value={name}
              onChange={e => setName(e.target.value)}
              className="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-indigo-500"
              placeholder="e.g. Ghost of Tsushima"
            />
          </div>
          
          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">Installation Folder</label>
            <div className="flex space-x-2">
              <input 
                type="text" 
                value={folder}
                onChange={e => setFolder(e.target.value)}
                className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-indigo-500"
                placeholder="D:\Games\Ghost of Tsushima"
              />
              <button type="button" onClick={handleSelectFolder} className="px-4 py-2 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-lg text-sm">Browse</button>
            </div>
          </div>
          
          <div>
            <label className="block text-sm font-medium text-gray-400 mb-1">Executable</label>
            <div className="flex space-x-2">
              <input 
                type="text" 
                value={executable}
                onChange={e => setExecutable(e.target.value)}
                className="flex-1 bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-white focus:outline-none focus:border-indigo-500"
                placeholder="D:\Games\Ghost of Tsushima\GhostOfTsushima.exe"
              />
              <button type="button" onClick={handleSelectExe} className="px-4 py-2 bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded-lg text-sm">Browse</button>
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
      </div>
    </div>
  );
}
