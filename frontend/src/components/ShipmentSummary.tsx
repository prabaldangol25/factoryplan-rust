import { useMemo } from 'react'
import type { Demand, Product, RunResult } from '../types'

interface Props {
  result: RunResult
  demand: Demand[]
  products: Product[]
}

interface Row {
  key: string
  label: string
  product: string
  demand: number
  shipped: number
  unshippable: number
}

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

export function ShipmentSummary({ result, demand, products }: Props) {
  const productName = useMemo(() => {
    const m = new Map<string, string>()
    for (const p of products) m.set(p.id, p.name)
    return m
  }, [products])

  const rows = useMemo<Row[]>(() => {
    const r: Row[] = []
    for (const d of demand) {
      const { label, key } = periodLabel(d)
      const units = result.units.filter((u) => u.demand_id === d.id)
      const shipped = units.filter((u) => u.status === 'shipped').length
      const unshippable = units.filter((u) => u.status === 'unshippable').length
      r.push({
        key: `${key}-${d.product_id}-${d.id}`,
        label,
        product: productName.get(d.product_id) ?? '(unknown)',
        demand: d.quantity,
        shipped,
        unshippable,
      })
    }
    r.sort((a, b) => a.label.localeCompare(b.label) || a.product.localeCompare(b.product))
    return r
  }, [demand, productName, result.units])

  const totals = rows.reduce(
    (acc, r) => {
      acc.demand += r.demand
      acc.shipped += r.shipped
      acc.unshippable += r.unshippable
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
            <th className="text-right px-3 py-2 font-medium">Demand</th>
            <th className="text-right px-3 py-2 font-medium">Shipped</th>
            <th className="text-right px-3 py-2 font-medium">Unshippable</th>
            <th className="text-right px-3 py-2 font-medium w-24">Fill %</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-slate-100">
          {rows.length === 0 && (
            <tr>
              <td colSpan={6} className="px-3 py-6 text-center text-slate-500">
                No demand to summarize.
              </td>
            </tr>
          )}
          {rows.map((r) => {
            const fill = r.demand > 0 ? (r.shipped / r.demand) * 100 : 0
            const color =
              fill >= 100
                ? 'text-emerald-700'
                : fill >= 75
                  ? 'text-amber-700'
                  : 'text-rose-700'
            return (
              <tr key={r.key}>
                <td className="px-3 py-2">{r.label}</td>
                <td className="px-3 py-2">{r.product}</td>
                <td className="px-3 py-2 text-right">{r.demand}</td>
                <td className="px-3 py-2 text-right text-emerald-700">{r.shipped}</td>
                <td className="px-3 py-2 text-right text-rose-700">{r.unshippable}</td>
                <td className={'px-3 py-2 text-right font-medium ' + color}>
                  {fill.toFixed(0)}%
                </td>
              </tr>
            )
          })}
        </tbody>
        {rows.length > 0 && (
          <tfoot className="bg-slate-50 font-medium">
            <tr>
              <td className="px-3 py-2" colSpan={2}>
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
