import { useEffect, useState } from 'react'
import { Plus, Trash2, Save, ChevronDown, ChevronRight } from 'lucide-react'
import type { Product } from '../types'
import * as api from '../api'

interface Props {
  scenarioId: string
}

interface LtCell {
  year: number
  quarter: number
  lead_time_days: number
}

function ltKey(year: number, quarter: number) {
  return `${year}-Q${quarter}`
}

export function ProductEditor({ scenarioId }: Props) {
  const [products, setProducts] = useState<Product[]>([])
  const [error, setError] = useState<string | null>(null)
  const [expanded, setExpanded] = useState<Record<string, boolean>>({})
  const [drafts, setDrafts] = useState<
    Record<string, { name: string; matrix: Record<string, LtCell> }>
  >({})
  const [newName, setNewName] = useState('')
  const [yearRange, setYearRange] = useState<{ from: number; to: number }>(() => {
    const y = new Date().getFullYear()
    return { from: y, to: y + 1 }
  })

  async function reload() {
    try {
      setError(null)
      const list = await api.listProducts(scenarioId)
      setProducts(list)
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'load failed')
    }
  }
  useEffect(() => {
    if (scenarioId) void reload()
  }, [scenarioId])

  function initDraft(p: Product) {
    const matrix: Record<string, LtCell> = {}
    for (const lt of p.lead_times) {
      matrix[ltKey(lt.year, lt.quarter)] = {
        year: lt.year,
        quarter: lt.quarter,
        lead_time_days: lt.lead_time_days,
      }
    }
    return { name: p.name, matrix }
  }

  function draftFor(p: Product) {
    return drafts[p.id] ?? initDraft(p)
  }
  function setDraft(
    id: string,
    fn: (d: { name: string; matrix: Record<string, LtCell> }) => { name: string; matrix: Record<string, LtCell> },
  ) {
    setDrafts((s) => {
      const current = s[id] ?? initDraft(products.find((p) => p.id === id)!)
      return { ...s, [id]: fn(current) }
    })
  }

  async function handleCreate() {
    if (!newName.trim()) return
    try {
      await api.createProduct(scenarioId, newName.trim(), [])
      setNewName('')
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'create failed')
    }
  }

  async function handleSave(p: Product) {
    const d = draftFor(p)
    const lts = Object.values(d.matrix).filter((c) => c.lead_time_days > 0)
    try {
      await api.updateProduct(p.id, d.name.trim(), lts)
      setDrafts((s) => {
        const x = { ...s }
        delete x[p.id]
        return x
      })
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'save failed')
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete product?')) return
    try {
      await api.deleteProduct(id)
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'delete failed')
    }
  }

  const years: number[] = []
  for (let y = yearRange.from; y <= yearRange.to; y++) years.push(y)

  return (
    <div className="space-y-4">
      {error && <div className="text-sm text-rose-600">{error}</div>}

      <div className="flex items-center gap-3 text-sm">
        <span className="text-slate-600">Lead-time matrix years:</span>
        <input
          type="number"
          className="w-20 border border-slate-300 rounded px-2 py-1"
          value={yearRange.from}
          onChange={(e) =>
            setYearRange((r) => ({ ...r, from: parseInt(e.target.value) || r.from }))
          }
        />
        <span className="text-slate-500">to</span>
        <input
          type="number"
          className="w-20 border border-slate-300 rounded px-2 py-1"
          value={yearRange.to}
          onChange={(e) => setYearRange((r) => ({ ...r, to: parseInt(e.target.value) || r.to }))}
        />
      </div>

      <div className="rounded-lg border border-slate-200 bg-white">
        {products.length === 0 && (
          <div className="px-3 py-6 text-center text-slate-500 text-sm">No products yet.</div>
        )}
        {products.map((p) => {
          const isOpen = expanded[p.id] ?? false
          const d = draftFor(p)
          const dirty =
            drafts[p.id] !== undefined &&
            JSON.stringify(drafts[p.id]) !== JSON.stringify(initDraft(p))
          return (
            <div key={p.id} className="border-b border-slate-100 last:border-b-0">
              <div className="flex items-center gap-2 px-3 py-2">
                <button
                  className="text-slate-500 hover:text-slate-800"
                  onClick={() => setExpanded((e) => ({ ...e, [p.id]: !isOpen }))}
                >
                  {isOpen ? (
                    <ChevronDown className="w-4 h-4" />
                  ) : (
                    <ChevronRight className="w-4 h-4" />
                  )}
                </button>
                <input
                  className="flex-1 border border-transparent hover:border-slate-200 rounded px-2 py-1"
                  value={d.name}
                  onChange={(e) => setDraft(p.id, (x) => ({ ...x, name: e.target.value }))}
                />
                {dirty && (
                  <button
                    className="text-emerald-700 hover:text-emerald-900"
                    onClick={() => handleSave(p)}
                    title="Save"
                  >
                    <Save className="w-4 h-4" />
                  </button>
                )}
                <button
                  className="text-rose-500 hover:text-rose-700"
                  onClick={() => handleDelete(p.id)}
                  title="Delete"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
              {isOpen && (
                <div className="px-3 pb-4 overflow-x-auto">
                  <table className="text-sm border border-slate-200">
                    <thead className="bg-slate-50">
                      <tr>
                        <th className="px-2 py-1 text-left font-medium text-slate-600">Year</th>
                        {[1, 2, 3, 4].map((q) => (
                          <th
                            key={q}
                            className="px-2 py-1 font-medium text-slate-600 w-24 text-center"
                          >
                            Q{q}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {years.map((y) => (
                        <tr key={y} className="border-t border-slate-200">
                          <td className="px-2 py-1 font-medium">{y}</td>
                          {[1, 2, 3, 4].map((q) => {
                            const k = ltKey(y, q)
                            const cell = d.matrix[k]
                            return (
                              <td key={q} className="px-1 py-1">
                                <input
                                  type="number"
                                  min={0}
                                  placeholder="days"
                                  className="w-20 border border-slate-200 rounded px-2 py-1 text-center"
                                  value={cell?.lead_time_days ?? ''}
                                  onChange={(e) => {
                                    const v = parseInt(e.target.value)
                                    setDraft(p.id, (x) => {
                                      const next = { ...x, matrix: { ...x.matrix } }
                                      if (!v || v <= 0) delete next.matrix[k]
                                      else
                                        next.matrix[k] = {
                                          year: y,
                                          quarter: q,
                                          lead_time_days: v,
                                        }
                                      return next
                                    })
                                  }}
                                />
                              </td>
                            )
                          })}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )}
            </div>
          )
        })}
        <div className="flex items-center gap-2 px-3 py-2 bg-slate-50">
          <input
            className="flex-1 border border-slate-300 rounded px-2 py-1 text-sm"
            placeholder="New product name…"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <button
            className="inline-flex items-center gap-1 px-3 py-1 bg-indigo-600 text-white text-sm rounded hover:bg-indigo-700"
            onClick={handleCreate}
          >
            <Plus className="w-4 h-4" />
            Add product
          </button>
        </div>
      </div>
    </div>
  )
}
