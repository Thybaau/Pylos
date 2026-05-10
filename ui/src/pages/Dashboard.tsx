import { useQuery } from '@tanstack/react-query'
import { logsApi } from '../lib/api'
import { StatCard } from '../components/StatCard'
import { RequestChart, TokenChart } from '../components/Charts'
import {
  formatLatency, formatCost, formatNumber, formatPercent,
} from '../lib/utils'
import { Activity, TrendingUp, Coins, Zap, Clock, Hash } from 'lucide-react'

const PERIOD = '24h'

export default function Dashboard() {
  const statsQ = useQuery({
    queryKey: ['logs-stats', PERIOD],
    queryFn: () => logsApi.getStats({ period: PERIOD }),
    refetchInterval: 30_000,
  })

  const histQ = useQuery({
    queryKey: ['histogram', PERIOD],
    queryFn: () => logsApi.getHistogram({ period: PERIOD }),
    refetchInterval: 30_000,
  })

  const tokensQ = useQuery({
    queryKey: ['token-histogram', PERIOD],
    queryFn: () => logsApi.getTokenHistogram({ period: PERIOD }),
    refetchInterval: 30_000,
  })

  const s = statsQ.data

  return (
    <div className="p-6 space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-white">Dashboard</h1>
          <p className="text-sm text-gray-400 mt-1">Last 24 hours</p>
        </div>
        <div className="flex items-center gap-2 text-xs text-gray-500">
          <div className="w-2 h-2 rounded-full bg-green-400 animate-pulse" />
          Live
        </div>
      </div>

      {/* KPI Cards */}
      <div className="grid grid-cols-2 md:grid-cols-3 xl:grid-cols-6 gap-4">
        <StatCard
          label="Total Requests"
          value={s ? formatNumber(s.total_requests) : '—'}
          color="blue"
          icon={<Activity size={14} />}
        />
        <StatCard
          label="Success Rate"
          value={s ? formatPercent(s.success_rate) : '—'}
          color={s && s.success_rate > 95 ? 'green' : 'red'}
          icon={<TrendingUp size={14} />}
        />
        <StatCard
          label="Avg Latency"
          value={s ? formatLatency(s.average_latency_ms) : '—'}
          color="yellow"
          icon={<Clock size={14} />}
        />
        <StatCard
          label="Total Tokens"
          value={s ? formatNumber(s.total_tokens) : '—'}
          color="purple"
          icon={<Hash size={14} />}
        />
        <StatCard
          label="Total Cost"
          value={s ? formatCost(s.total_cost_usd) : '—'}
          color="yellow"
          icon={<Coins size={14} />}
        />
        <StatCard
          label="Prompt / Completion"
          value={s ? `${formatNumber(s.total_prompt_tokens)} / ${formatNumber(s.total_completion_tokens)}` : '—'}
          color="blue"
          icon={<Zap size={14} />}
        />
      </div>

      {/* Charts */}
      <div className="grid grid-cols-1 xl:grid-cols-2 gap-6">
        {/* Request Volume */}
        <div className="rounded-xl border border-gray-800 bg-gray-900 p-5">
          <h2 className="text-sm font-semibold text-gray-300 mb-4">Request Volume</h2>
          {histQ.data?.buckets.length ? (
            <RequestChart
              buckets={histQ.data.buckets}
              bucketSecs={histQ.data.bucket_size_seconds}
            />
          ) : (
            <EmptyChart />
          )}
        </div>

        {/* Token Usage */}
        <div className="rounded-xl border border-gray-800 bg-gray-900 p-5">
          <h2 className="text-sm font-semibold text-gray-300 mb-4">Token Usage</h2>
          {tokensQ.data?.buckets.length ? (
            <TokenChart
              buckets={tokensQ.data.buckets}
              bucketSecs={tokensQ.data.bucket_size_seconds}
            />
          ) : (
            <EmptyChart />
          )}
        </div>
      </div>
    </div>
  )
}

function EmptyChart() {
  return (
    <div className="h-[220px] flex items-center justify-center text-gray-600 text-sm">
      No data yet — send some requests to see charts
    </div>
  )
}
