import { useEffect, useMemo, useState } from 'react'
import { Plus, Trash2 } from 'lucide-react'
import type { Demand, PeriodType, Product, SpreadMode } from '../types'
import * as api from '../api'

interface Props {
  scenarioId: string
}

interface NewDemandForm {
  product_id: string
  period_type: PeriodType
  year: number
  period_index: number
  quantity: number
  spread_mode: SpreadMode
}

const MONTH_NAMES = [
  'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
]

export function DemandEditor({ scenarioId }: Props) {
  const [demand, setDemand] = useState<Demand[]>([])
  const [products, setProducts] = useState<Product[]>([])
  const [error, setError] = useState<string | null>(null)

  const [form, setForm] = useState<NewDemandForm>(() => ({
    product_id: '',
    period_type: 'quarter',
    year: new Date().getFullYear(),
    period_index: 1,
    quantity: 10,
    spread_mode: 'even',
  }))

  async function reload() {
    try {
      setError(null)
      const [d, p] = await Promise.all([api.listDemand(scenarioId), api.listProducts(scenarioId)])
      setDemand(d)
      setProducts(p)
      if (!form.product_id && p.length > 0) {
        setForm((f) => ({ ...f, product_id: p[0].id }))
      }
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'load failed')
    }
  }
  useEffect(() => {
    if (scenarioId) void reload()
  }, [scenarioId])

  const productMap = useMemo(() => {
    const m = new Map<string, Product>()
    for (const p of products) m.set(p.id, p)
    return m
  }, [products])

  async function handleCreate() {
    if (!form.product_id) {
      setError('select a product first')
      return
    }
    try {
      await api.createDemand(scenarioId, form)
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'create failed')
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete demand row?')) return
    try {
      await api.deleteDemand(id)
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'delete failed')
    }
  }

  function periodLabel(d: Demand) {
    if (d.period_type === 'quarter') return `${d.year} · Q${d.period_index}`
    return `${MONTH_NAMES[d.period_index - 1]} ${d.year}`
  }

  return (
    <div className="space-y-4">
      {error && <div className="text-sm text-rose-600">{error}</div>}

      <div className="rounded-lg border border-slate-200 bg-white overflow-hidden">
        <table className="min-w-full text-sm">
          <thead className="bg-slate-50 text-slate-600">
            <tr>
              <th className="text-left px-3 py-2 font-medium">Product</th>
              <th className="text-left px-3 py-2 font-medium">Period</th>
              <th className="text-left px-3 py-2 font-medium w-24">Quantity</th>
              <th className="text-left px-3 py-2 font-medium w-28">Spread</th>
              <th className="w-12"></th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {demand.length === 0 && (
              <tr>
                <td colSpan={5} className="px-3 py-6 text-center text-slate-500">
                  No demand rows yet.
                </td>
              </tr>
            )}
            {demand.map((d) => (
              <tr key={d.id}>
                <td className="px-3 py-2">{productMap.get(d.product_id)?.name ?? '(unknown)'}</td>
                <td className="px-3 py-2">{periodLabel(d)}</td>
                <td className="px-3 py-2">{d.quantity}</td>
                <td className="px-3 py-2 text-slate-600">{d.spread_mode}</td>
                <td className="px-3 py-2 text-right">
                  <button
                    className="text-rose-500 hover:text-rose-700"
                    onClick={() => handleDelete(d.id)}
                    title="Delete"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* New demand form */}
        <div className="border-t border-slate-200 bg-slate-50 px-3 py-3 flex items-end gap-3 flex-wrap">
          <div>
            <label className="block text-xs text-slate-500 mb-1">Product</label>
            <select
              className="border border-slate-300 rounded px-2 py-1 text-sm bg-white min-w-[10rem]"
              value={form.product_id}
              onChange={(e) => setForm((f) => ({ ...f, product_id: e.target.value }))}
            >
              <option value="">(select)</option>
              {products.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-slate-500 mb-1">Type</label>
            <select
              className="border border-slate-300 rounded px-2 py-1 text-sm bg-white"
              value={form.period_type}
              onChange={(e) =>
                setForm((f) => ({
                  ...f,
                  period_type: e.target.value as PeriodType,
                  period_index: 1,
                }))
              }
            >
              <option value="quarter">Quarter</option>
              <option value="month">Month</option>
            </select>
          </div>
          <div>
            <label className="block text-xs text-slate-500 mb-1">Year</label>
            <input
              type="number"
              className="w-24 border border-slate-300 rounded px-2 py-1 text-sm"
              value={form.year}
              onChange={(e) => setForm((f) => ({ ...f, year: parseInt(e.target.value) || f.year }))}
            />
          </div>
          <div>
            <label className="block text-xs text-slate-500 mb-1">
              {form.period_type === 'quarter' ? 'Quarter' : 'Month'}
            </label>
            <select
              className="border border-slate-300 rounded px-2 py-1 text-sm bg-white"
              value={form.period_index}
              onChange={(e) =>
                setForm((f) => ({ ...f, period_index: parseInt(e.target.value) }))
              }
            >
              {form.period_type === 'quarter'
                ? [1, 2, 3, 4].map((q) => (
                    <option key={q} value={q}>
                      Q{q}
                    </option>
                  ))
                : MONTH_NAMES.map((m, i) => (
                    <option key={m} value={i + 1}>
                      {m}
                    </option>
                  ))}
            </select>
          </div>
          <div>
            <label className="block text-xs text-slate-500 mb-1">Quantity</label>
            <input
              type="number"
              min={1}
              className="w-20 border border-slate-300 rounded px-2 py-1 text-sm"
              value={form.quantity}
              onChange={(e) =>
                setForm((f) => ({ ...f, quantity: parseInt(e.target.value) || 1 }))
              }
            />
          </div>
          <div>
            <label className="block text-xs text-slate-500 mb-1">Spread</label>
            <select
              className="border border-slate-300 rounded px-2 py-1 text-sm bg-white"
              value={form.spread_mode}
              onChange={(e) =>
                setForm((f) => ({ ...f, spread_mode: e.target.value as SpreadMode }))
              }
            >
              <option value="even">Even</option>
              <option value="start">Start of period</option>
              <option value="end">End of period</option>
            </select>
          </div>
          <button
            className="inline-flex items-center gap-1 px-3 py-1.5 bg-indigo-600 text-white text-sm rounded hover:bg-indigo-700"
            onClick={handleCreate}
          >
            <Plus className="w-4 h-4" />
            Add
          </button>
        </div>
      </div>
    </div>
  )
}
