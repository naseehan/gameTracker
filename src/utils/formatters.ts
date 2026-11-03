/** A duration like "12h 5m", "45m" or "under a minute". */
export function formatPlaytime(seconds: number): string {
  if (seconds <= 0) return '0m';
  if (seconds < 60) return '<1m';

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);

  if (hours > 0) {
    if (minutes > 0) return `${hours}h ${minutes}m`;
    return `${hours}h`;
  }

  return `${minutes}m`;
}

/** A stopwatch-style duration: "4:09" or "1:04:09". */
export function formatClock(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const pad = (n: number) => n.toString().padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

export function formatRelativeTime(dateString: string | null): string {
  if (!dateString) return 'Never';

  const date = new Date(dateString);
  const now = new Date();
  const diffInSeconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (diffInSeconds < 60) return 'Just now';
  if (diffInSeconds < 3600) return `${Math.floor(diffInSeconds / 60)}m ago`;
  if (diffInSeconds < 86400) return `${Math.floor(diffInSeconds / 3600)}h ago`;
  if (diffInSeconds < 172800) return 'Yesterday';

  const diffInDays = Math.floor(diffInSeconds / 86400);
  if (diffInDays < 30) return `${diffInDays}d ago`;

  return date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

export function formatTime(dateString: string): string {
  return new Date(dateString).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
}

export function formatShortDate(dateString: string): string {
  return new Date(dateString).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
}

/** "Today", "Yesterday" or a date like "Mon, Oct 20", for a local calendar day. */
export function formatDayHeading(date: Date): string {
  const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((startOfDay(new Date()) - startOfDay(date)) / 86400000);
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  return date.toLocaleDateString(undefined, {
    weekday: 'short',
    month: 'short',
    day: 'numeric',
    ...(date.getFullYear() !== new Date().getFullYear() ? { year: 'numeric' } : {}),
  });
}

/** The value for an <input type="datetime-local"> showing this moment in local time. */
export function toDateTimeLocal(dateString: string): string {
  const d = new Date(dateString);
  const pad = (n: number) => n.toString().padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Chart axis label for a bucket key: "2026-10-27" → "10/27", "2026-10" → "Oct '26", "14:00" stays. */
export function formatChartLabel(label: string): string {
  const parts = label.split('-');
  if (parts.length === 3) return `${parts[1]}/${parts[2]}`;
  if (parts.length === 2) {
    const date = new Date(Number(parts[0]), Number(parts[1]) - 1, 1);
    return `${date.toLocaleDateString(undefined, { month: 'short' })} '${parts[0].slice(2)}`;
  }
  return label;
}

/** Tooltip formatter for charts whose data rows carry a `minutes` field. */
export function chartTooltip(_value: unknown, _name: unknown, props: { payload?: { minutes?: number } }) {
  const minutes = props.payload?.minutes ?? 0;
  return [formatPlaytime(minutes * 60), 'Playtime'] as [string, string];
}
