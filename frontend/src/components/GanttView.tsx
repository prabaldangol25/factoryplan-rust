import { useMemo, useState } from 'react'
import Plot from 'react-plotly.js'
import type { Factory, Product, RunResult } from '../types'

interface Props {
  result: RunResult
  factories: Factory[]
  products: Product[]
}

const PALETTE = [
  '#4f46e5', '#0891b2', '#059669', '#d97706', '#dc2626',
  '#7c3aed', '#db2777', '#0ea5e9', '#65a30d', '#ea580c',
]

function colorForProduct(idx: number): string {
  return PALETTE[idx % PALETTE.length]
}

export function GanttView({ result, factories, products }: Props) {
  const [factoryId, setFactoryId] = useState<string>(() => factories[0]?.id ?? '')

  const productColors = useMemo(() => {
    const m = new Map<string, string>()
    products.forEach((p, i) => m.set(p.id, colorForProduct(i)))
    return m
  }, [products])

  const productNames = useMemo(() => {
    const m = new Map<string, string>()
    products.forEach((p) => m.set(p.id, p.name))
    return m
  }, [products])

  const factory = factories.find((f) => f.id === factoryId)

  const { data, layout } = useMemo(() => {
    if (!factory) return { data: [] as Plotly.Data[], layout: {} as Partial<Plotly.Layout> }

    const units = result.units.filter(
      (u) => u.status === 'shipped' && u.factory_id === factory.id,
    )

    // Build one trace per product (so legend groups by product)
    const byProduct = new Map<string, typeof units>()
    for (const u of units) {
      const arr = byProduct.get(u.product_id) ?? []
      arr.push(u)
      byProduct.set(u.product_id, arr)
    }

    const traces: Plotly.Data[] = []
    for (const [pid, list] of byProduct.entries()) {
      const color = productColors.get(pid) ?? '#6b7280'
      const name = productNames.get(pid) ?? pid
      traces.push({
        type: 'bar',
        orientation: 'h',
        name,
        x: list.map(
          (u) =>
            new Date(u.due_date).getTime() - new Date(u.required_start).getTime() + 86400000,
        ),
        y: list.map((u) => `Bay ${(u.bay_index ?? 0) + 1}`),
        base: list.map((u) => new Date(u.required_start).getTime()),
        marker: { color },
        hovertemplate: list
          .map(
            (u) =>
              `${name}<br>Bay ${(u.bay_index ?? 0) + 1}<br>${u.required_start} → ${u.due_date}<extra></extra>`,
          )
          .reduce<string[]>((acc, t) => {
            acc.push(t)
            return acc
          }, []),
        text: list.map(() => ''),
      } as Plotly.Data)
    }

    const bayLabels: string[] = []
    for (let i = 1; i <= factory.bays; i++) bayLabels.push(`Bay ${i}`)

    const layout: Partial<Plotly.Layout> = {
      barmode: 'stack',
      xaxis: {
        type: 'date',
        title: { text: 'Date' },
        showgrid: true,
      },
      yaxis: {
        title: { text: 'Bays' },
        type: 'category',
        categoryorder: 'array',
        categoryarray: bayLabels,
        autorange: 'reversed',
      },
      margin: { l: 80, r: 20, t: 20, b: 50 },
      legend: { orientation: 'h', y: -0.2 },
      height: Math.max(220, factory.bays * 32 + 140),
    }

    return { data: traces, layout }
  }, [factory, productColors, productNames, result.units])

  if (factories.length === 0) {
    return (
      <div className="rounded-lg border border-dashed border-slate-300 p-6 text-center text-slate-500 text-sm">
        No factories defined.
      </div>
    )
  }

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2 text-sm">
        <span className="text-slate-600">Factory:</span>
        <select
          className="border border-slate-300 rounded px-2 py-1 bg-white"
          value={factoryId}
          onChange={(e) => setFactoryId(e.target.value)}
        >
          {factories.map((f) => (
            <option key={f.id} value={f.id}>
              {f.name} ({f.bays} bays)
            </option>
          ))}
        </select>
      </div>
      <div className="rounded-lg border border-slate-200 bg-white p-3">
        <Plot
          data={data}
          layout={layout}
          config={{ displayModeBar: false, responsive: true }}
          style={{ width: '100%' }}
          useResizeHandler
        />
      </div>
    </div>
  )
}
