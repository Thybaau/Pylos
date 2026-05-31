import {
  BarChart, Bar, LineChart, Line, XAxis, YAxis, CartesianGrid,
  Tooltip, ResponsiveContainer, Legend,
} from 'recharts'
import { format } from 'date-fns'
import type { HistogramBucket, TokenBucket } from '../lib/api'

const tooltipStyle = {
  background: '#18181b',
  border: '1px solid rgba(63, 63, 70, 0.5)',
  borderRadius: 8,
}

interface RequestChartProps {
  buckets: HistogramBucket[]
  bucketSecs: number
}

export function RequestChart({ buckets, bucketSecs }: RequestChartProps) {
  const data = buckets.map(b => ({
    time: format(new Date(b.timestamp), bucketSecs < 3600 ? 'HH:mm' : 'MM/dd HH:mm'),
    success: b.success,
    error: b.error,
  }))

  return (
    <ResponsiveContainer width="100%" height={220}>
      <BarChart data={data} barSize={6} barGap={2}>
        <CartesianGrid stroke="rgba(63, 63, 70, 0.5)" />
        <XAxis dataKey="time" tick={{ fill: '#71717a', fontSize: 11 }} />
        <YAxis tick={{ fill: '#71717a', fontSize: 11 }} />
        <Tooltip
          contentStyle={tooltipStyle}
          labelStyle={{ color: '#a1a1aa' }}
        />
        <Legend wrapperStyle={{ fontSize: 12, color: '#a1a1aa' }} />
        <Bar dataKey="success" fill="#10b981" name="Success" radius={[2, 2, 0, 0]} />
        <Bar dataKey="error"   fill="#f43f5e" name="Error"   radius={[2, 2, 0, 0]} />
      </BarChart>
    </ResponsiveContainer>
  )
}

interface TokenChartProps {
  buckets: TokenBucket[]
  bucketSecs: number
}

export function TokenChart({ buckets, bucketSecs }: TokenChartProps) {
  const data = buckets.map(b => ({
    time: format(new Date(b.timestamp), bucketSecs < 3600 ? 'HH:mm' : 'MM/dd HH:mm'),
    prompt: b.prompt_tokens,
    completion: b.completion_tokens,
  }))

  return (
    <ResponsiveContainer width="100%" height={220}>
      <LineChart data={data}>
        <CartesianGrid stroke="rgba(63, 63, 70, 0.5)" />
        <XAxis dataKey="time" tick={{ fill: '#71717a', fontSize: 11 }} />
        <YAxis tick={{ fill: '#71717a', fontSize: 11 }} />
        <Tooltip
          contentStyle={tooltipStyle}
          labelStyle={{ color: '#a1a1aa' }}
        />
        <Legend wrapperStyle={{ fontSize: 12, color: '#a1a1aa' }} />
        <Line
          dataKey="prompt"
          stroke="#3b82f6"
          name="Prompt"
          dot={false}
          strokeWidth={2}
        />
        <Line
          dataKey="completion"
          stroke="#10b981"
          name="Completion"
          dot={false}
          strokeWidth={2}
        />
      </LineChart>
    </ResponsiveContainer>
  )
}
