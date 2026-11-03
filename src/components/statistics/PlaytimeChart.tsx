import { ResponsiveContainer, BarChart, Bar, XAxis, YAxis, Tooltip, CartesianGrid } from 'recharts';
import { DailyPlaytime } from '../../types';
import { chartTooltip, formatChartLabel } from '../../utils/formatters';
import { CHART_COLOR, chartTooltipStyle } from './chartTheme';

interface PlaytimeChartProps {
  data: DailyPlaytime[];
  title?: string;
  /** Render just the chart, for use inside another card. */
  bare?: boolean;
}

export function PlaytimeChart({ data, title = 'Playtime Over Time', bare = false }: PlaytimeChartProps) {
  const formattedData = data.map(d => ({
    ...d,
    hours: Number((d.playtime_seconds / 3600).toFixed(2)),
    minutes: Math.floor(d.playtime_seconds / 60)
  }));

  const chart = (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={formattedData} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" stroke="#374151" vertical={false} />
        <XAxis
          dataKey="date_label"
          stroke="#9CA3AF"
          fontSize={12}
          tickLine={false}
          axisLine={false}
          tickFormatter={formatChartLabel}
          minTickGap={12}
        />
        <YAxis
          stroke="#9CA3AF"
          fontSize={12}
          tickLine={false}
          axisLine={false}
          allowDecimals={false}
          tickFormatter={(val) => `${val}h`}
        />
        <Tooltip {...chartTooltipStyle} labelFormatter={(label) => formatChartLabel(String(label))} formatter={chartTooltip} />
        <Bar dataKey="hours" fill={CHART_COLOR} radius={[4, 4, 0, 0]} />
      </BarChart>
    </ResponsiveContainer>
  );

  if (bare) return <div className="w-full h-full">{chart}</div>;

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 h-[300px] flex flex-col">
      <h3 className="text-gray-400 font-medium text-sm mb-4">{title}</h3>
      <div className="flex-1 w-full h-full min-h-0">{chart}</div>
    </div>
  );
}
