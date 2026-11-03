import { useEffect, useState } from 'react';
import { formatClock } from '../utils/formatters';

/** Time since `startedAt`, updated every second (e.g. "1:04:09"). */
export function LiveTimer({ startedAt, className = '' }: { startedAt: string; className?: string }) {
  const [now, setNow] = useState(Date.now());

  useEffect(() => {
    const interval = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(interval);
  }, []);

  const elapsed = Math.max(0, Math.floor((now - new Date(startedAt).getTime()) / 1000));
  return <span className={`font-mono tabular-nums ${className}`}>{formatClock(elapsed)}</span>;
}
