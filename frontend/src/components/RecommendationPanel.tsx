import { Building2, Gauge, Package } from 'lucide-react'
import type { Recommendation } from '../types'

interface Props {
  recommendation: Recommendation
  totalDemand: number
  shipped: number
  unshippable: number
}

function Card({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode
  title: string
  children: React.ReactNode
}) {
  return (
    <div className="rounded-lg border border-slate-200 bg-white p-4">
      <div className="flex items-center gap-2 text-slate-700 mb-2">
        {icon}
        <span className="font-medium text-sm">{title}</span>
      </div>
      <div className="text-sm text-slate-700">{children}</div>
    </div>
  )
}

export function RecommendationPanel({
  recommendation,
  totalDemand,
  shipped,
  unshippable,
}: Props) {
  const allClear = unshippable === 0
  if (allClear) {
    return (
      <div className="rounded-lg border border-emerald-200 bg-emerald-50 p-4 text-sm text-emerald-800">
        <div className="font-medium">All {totalDemand} units ship on time.</div>
        <div className="text-emerald-700 mt-1">
          {shipped} shipped, 0 unshippable. No recommendations needed.
        </div>
      </div>
    )
  }

  const { bays_needed, uniform_lt_pct, per_product_lt } = recommendation
  const noRecommendable =
    !bays_needed && !uniform_lt_pct && (per_product_lt?.length ?? 0) === 0

  return (
    <div className="space-y-3">
      <div className="rounded-lg border border-rose-200 bg-rose-50 p-4 text-sm text-rose-800">
        <div className="font-medium">
          Shortfall: {unshippable} unshippable of {totalDemand}.
        </div>
        <div className="text-rose-700 mt-1">
          {shipped} shipped on time. Pick a lever below.
        </div>
      </div>

      {noRecommendable && (
        <div className="rounded-lg border border-amber-200 bg-amber-50 p-4 text-sm text-amber-900">
          No single recommendation could clear the shortfall — try increasing bay counts and
          reducing lead times together, or revisit the demand plan.
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
        {bays_needed && (
          <Card icon={<Building2 className="w-4 h-4" />} title="Add bays">
            <div className="text-2xl font-semibold">+{bays_needed.bays_to_add}</div>
            <div className="text-slate-500">
              bays at{' '}
              <span className="font-medium text-slate-700">
                {bays_needed.suggested_factory_name ?? '(any factory)'}
              </span>
            </div>
          </Card>
        )}
        {uniform_lt_pct && (
          <Card icon={<Gauge className="w-4 h-4" />} title="Reduce all lead times">
            <div className="text-2xl font-semibold">
              −{uniform_lt_pct.reduction_pct.toFixed(1)}%
            </div>
            <div className="text-slate-500">across every product</div>
          </Card>
        )}
        {per_product_lt && per_product_lt.length > 0 && (
          <Card icon={<Package className="w-4 h-4" />} title="Target product lead times">
            <ul className="space-y-1">
              {per_product_lt.map((p) => (
                <li key={p.product_id} className="flex items-center justify-between gap-2">
                  <span>{p.product_name}</span>
                  <span className="text-slate-600">
                    <span className="line-through text-slate-400 mr-1">
                      {p.current_lead_time_days}d
                    </span>
                    <span className="font-medium">{p.target_lead_time_days}d</span>
                  </span>
                </li>
              ))}
            </ul>
          </Card>
        )}
      </div>
    </div>
  )
}
