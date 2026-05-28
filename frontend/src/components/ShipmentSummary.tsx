import { useMemo } from 'react'
import type { Demand, Factory, Product, RunResult } from '../types'

interface Props {
  result: RunResult
  demand: Demand[]
  products: Product[]
  factories: Factory[]
}

interface SubRow {
  factory: string // factory name, or "(unshippable)"
  isUnshippable: boolean
  shipped: number
  unshippable: number
}

interface Group {
  key: string
  label: string
  product: string
  demand: number
  subrows: SubRow[]
}

const UNSHIPPABLE_LABEL = '(unshippable)'

function periodLabel(d: Demand): { label: string; key: string } {
  if (d.period_type === 'quarter') {
    return { label: `${d.year} · Q${d.period_index}`, key: `${d.year}-Q${d.period_index}` }
  }
  const months = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec']
  return {
    label: `${months[d.period_index - 1]} ${d.year}`,
    key: `${d.year}-M${d.period_index.toString().padStart(2, '0')}`,
  }
}

export function ShipmentSummary({ result, demand, products, factories }: Props) {
  const productName = useMemo(() => {
    const m = new Map<string, string>()
    for (const p of products) m.set(p.id, p.name)
    return m
  }, [products])

  const factoryName = useMemo(() => {
    const m = new Map<string, string>()
    for (const f of factories) m.set(f.id, f.name)
    return m
  }, [factories])

  const groups = useMemo<Group[]>(() => {
    const out: Group[] = []
    for (const d of demand) {
      const { label, key } = periodLabel(d)
      const units = result.units.filter((u) => u.demand_id === d.id)

      // Group units by factory (or "(unshippable)")
      const byFactory = new Map<string, SubRow>()
      // Seed with every factory so empty ones are visible
      for (const f of factories) {
        byFactory.set(f.id, {
          factory: f.name,
          isUnshippable: false,
          shipped: 0,
          unshippable: 0,
        })
      }
      let unshippableCount = 0
      for (const u of units) {
        if (u.status === 'unshippable' || u.factory_id == null) {
          unshippableCount += 1
          continue
        }
        const row = byFactory.get(u.factory_id) ?? {
          factory: factoryName.get(u.factory_id) ?? '(unknown)',
          isUnshippable: false,
          shipped: 0,
          unshippable: 0,
        }
        row.shipped += 1
        byFactory.set(u.factory_id, row)
      }
      // Keep factory order stable (matches factories prop order)
      const subrows: SubRow[] = []
      for (const f of factories) {
        const r = byFactory.get(f.id)
        if (r && (r.shipped > 0 || r.unshippable > 0)) {
          subrows.push(r)
        }
      }
      // Factories not in the prop list (defensive)
      for (const [fid, r] of byFactory.entries()) {
        if (!factories.find((f) => f.id === fid) && (r.shipped > 0 || r.unshippable > 0)) {
          subrows.push(r)
        }
      }
      if (unshippableCount > 0) {
        subrows.push({
          factory: UNSHIPPABLE_LABEL,
          isUnshippable: true,
          shipped: 0,
          unshippable: unshippableCount,
        })
      }
      // If literally nothing landed anywhere (e.g. zero quantity edge case), still show one row
      if (subrows.length === 0) {
        subrows.push({
          factory: UNSHIPPABLE_LABEL,
          isUnshippable: true,
          shipped: 0,
          unshippable: d.quantity,
        })
      }

      out.push({
        key: `${key}-${d.product_id}-${d.id}`,
        label,
        product: productName.get(d.product_id) ?? '(unknown)',
        demand: d.quantity,
        subrows,
      })
    }
    out.sort((a, b) => a.label.localeCompare(b.label) || a.product.localeCompare(b.product))
    return out
  }, [demand, factories, factoryName, productName, result.units])

  // Grand totals
  const totals = groups.reduce(
    (acc, g) => {
      const shipped = g.subrows.reduce((s, r) => s + r.shipped, 0)
      const unshippable = g.subrows.reduce((s, r) => s + r.unshippable, 0)
      acc.demand += g.demand
      acc.shipped += shipped
      acc.unshippable += unshippable
      return acc
    },
    { demand: 0, shipped: 0, unshippable: 0 },
  )
  const totalsFill = totals.demand > 0 ? (totals.shipped / totals.demand) * 100 : 0

  return (
    <div className="rounded-lg border border-slate-200 bg-white overflow-hidden">
      <table className="min-w-full text-sm">
        <thead className="bg-slate-50 text-slate-600">
          <tr>
            <th className="text-left px-3 py-2 font-medium">Period</th>
            <th className="text-left px-3 py-2 font-medium">Product</th>
            <th className="text-left px-3 py-2 font-medium">Factory</th>
            <th className="text-right px-3 py-2 font-medium">Demand</th>
            <th className="text-right px-3 py-2 font-medium">Shipped</th>
            <th className="text-right px-3 py-2 font-medium">Unshippable</th>
            <th className="text-right px-3 py-2 font-medium w-24">Fill %</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-100">
          {groups.length === 0 && (
            <tr>
              <td colSpan={7} className="px-3 py-6 text-center text-slate-500">
                No demand to summarize.
              </td>
            </tr>
          )}
          {groups.map((g) => {
            const groupShipped = g.subrows.reduce((s, r) => s + r.shipped, 0)
            const groupFill = g.demand > 0 ? (groupShipped / g.demand) * 100 : 0
            const fillColor =
              groupFill >= 100
                ? 'text-emerald-700'
                : groupFill >= 75
                  ? 'text-amber-700'
                  : 'text-rose-700'
            return g.subrows.map((sub, i) => (
              <tr key={`${g.key}-${i}`} className={i > 0 ? 'border-t-0' : ''}>
                {i === 0 && (
                  <td
                    rowSpan={g.subrows.length}
                    className="px-3 py-2 align-top border-t border-slate-100"
                  >
                    {g.label}
                  </td>
                )}
                {i === 0 && (
                  <td
                    rowSpan={g.subrows.length}
                    className="px-3 py-2 align-top border-t border-slate-100"
                  >
                    {g.product}
                  </td>
                )}
                <td
                  className={
                    'px-3 py-2 ' +
                    (sub.isUnshippable ? 'text-rose-700 italic' : 'text-slate-700')
                  }
                >
                  {sub.factory}
                </td>
                {i === 0 && (
                  <td
                    rowSpan={g.subrows.length}
                    className="px-3 py-2 text-right align-top border-t border-slate-100"
                  >
                    {g.demand}
                  </td>
                )}
                <td className="px-3 py-2 text-right text-emerald-700">
                  {sub.shipped > 0 ? sub.shipped : ''}
                </td>
                <td className="px-3 py-2 text-right text-rose-700">
                  {sub.unshippable > 0 ? sub.unshippable : ''}
                </td>
                {i === 0 && (
                  <td
                    rowSpan={g.subrows.length}
                    className={
                      'px-3 py-2 text-right font-medium align-top border-t border-slate-100 ' +
                      fillColor
                    }
                  >
                    {groupFill.toFixed(0)}%
                  </td>
                )}
              </tr>
            ))
          })}
        </tbody>
        {groups.length > 0 && (
          <tfoot className="bg-slate-50 font-medium">
            <tr>
              <td className="px-3 py-2" colSpan={3}>
                Total
              </td>
              <td className="px-3 py-2 text-right">{totals.demand}</td>
              <td className="px-3 py-2 text-right text-emerald-700">{totals.shipped}</td>
              <td className="px-3 py-2 text-right text-rose-700">{totals.unshippable}</td>
              <td className="px-3 py-2 text-right">{totalsFill.toFixed(0)}%</td>
            </tr>
          </tfoot>
        )}
      </table>
    </div>
  )
}
