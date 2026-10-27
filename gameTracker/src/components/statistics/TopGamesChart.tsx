import { ResponsiveContainer, BarChart, Bar, XAxis, YAxis, Tooltip, CartesianGrid } from 'recharts';
import { TopGame } from '../../store/statisticsStore';
import { useNavigationStore } from '../../store/navigationStore';

interface TopGamesChartProps {
  data: TopGame[];
}

export function TopGamesChart({ data }: TopGamesChartProps) {
  const { navigate } = useNavigationStore();
  const formattedData = data.map(d => ({
    id: d.game_id,
    name: d.name,
    hours: Number((d.playtime_seconds / 3600).toFixed(2)),
    minutes: Math.floor(d.playtime_seconds / 60)
  }));

  const formatTooltip = (_value: any, _name: any, props: any) => {
    const mins = props.payload.minutes;
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return [`${h > 0 ? `${h}h ` : ''}${m}m`, 'Playtime'];
  };

  const handleBarClick = (data: any) => {
    if (data && data.id) {
      navigate({ type: 'game_details', gameId: data.id });
    }
  };

  return (
    <div className="bg-gray-900 border border-gray-800 rounded-xl p-5 h-[300px] flex flex-col">
      <h3 className="text-gray-400 font-medium text-sm mb-4">Most Played Games</h3>
      <div className="flex-1 w-full h-full min-h-0">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={formattedData} layout="vertical" margin={{ top: 0, right: 10, left: 10, bottom: 0 }}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" horizontal={false} />
            <XAxis 
              type="number"
              stroke="#9CA3AF" 
              fontSize={12} 
              tickLine={false} 
              axisLine={false}
              tickFormatter={(val) => `${val}h`}
            />
            <YAxis 
              dataKey="name" 
              type="category"
              stroke="#9CA3AF" 
              fontSize={12} 
              tickLine={false} 
              axisLine={false}
              width={100}
              tickFormatter={(val) => val.length > 12 ? val.substring(0, 10) + '...' : val}
            />
            <Tooltip 
              contentStyle={{ backgroundColor: '#111827', borderColor: '#374151', borderRadius: '0.5rem' }}
              itemStyle={{ color: '#E5E7EB' }}
              formatter={formatTooltip}
              cursor={{ fill: '#374151', opacity: 0.4 }}
            />
            <Bar dataKey="hours" fill="#8b5cf6" radius={[0, 4, 4, 0]} onClick={handleBarClick} className="cursor-pointer" />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}