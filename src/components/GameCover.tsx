import { ReactNode, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { Gamepad2 } from 'lucide-react';

interface GameCoverProps {
  path: string | null;
  alt: string;
  className?: string;
  iconSize?: number;
  /** Shown when there is no cover or it fails to load. Defaults to a gamepad icon. */
  fallback?: ReactNode;
}

/** A game's cover image, served from the app data folder through Tauri's asset protocol. */
export function GameCover({ path, alt, className = '', iconSize = 48, fallback }: GameCoverProps) {
  const [failedPath, setFailedPath] = useState<string | null>(null);

  if (!path || failedPath === path) {
    if (fallback !== undefined) return <>{fallback}</>;
    return (
      <div className={`flex items-center justify-center bg-gray-800 text-gray-600 ${className}`}>
        <Gamepad2 size={iconSize} />
      </div>
    );
  }

  return (
    <img
      src={convertFileSrc(path)}
      alt={alt}
      className={`object-cover ${className}`}
      onError={() => setFailedPath(path)}
    />
  );
}
