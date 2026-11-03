import { ResponsiveContainer, BarChart, Bar, XAxis, YAxis, Tooltip, CartesianGrid } from 'recharts';
import { DayOfWeekActivity } from '../../store/statisticsStore';
import { chartTooltip } from '../../utils/formatters';
import { CHART_COLOR, chartTooltipStyle } from './chartTheme';

interface ActivityChartProps {
  data: DayOfWeekActivity[];
}

export function ActivityChart({ data }: ActivityChartProps) {
  // The backend returns the days in order, Monday first.
  const formattedData = data.map(d => ({
    day: d.day,
    hours: Number((d.playtime_seconds / 3600).toFixed(2)),
    minutes: Math.floor(d.playtime_seconds / 60)
  }));

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 h-[300px] flex flex-col">
      <h3 className="text-gray-400 font-medium text-sm mb-4">Activity by Day of the Week</h3>
      <div className="flex-1 w-full h-full min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={formattedData} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" vertical={false} />
            <XAxis dataKey="day" stroke="#9CA3AF" fontSize={12} tickLine={false} axisLine={false} />
            <YAxis
              stroke="#9CA3AF"
              fontSize={12}
              tickLine={false}
              axisLine={false}
              allowDecimals={false}
              tickFormatter={(val) => `${val}h`}
            />
            <Tooltip {...chartTooltipStyle} formatter={chartTooltip} />
            <Bar dataKey="hours" fill={CHART_COLOR} radius={[4, 4, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
