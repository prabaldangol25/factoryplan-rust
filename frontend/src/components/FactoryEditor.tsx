import { useEffect, useState } from 'react'
import { Plus, Trash2, Save, ChevronDown, ChevronRight } from 'lucide-react'
import type { Factory } from '../types'
import * as api from '../api'

interface Props {
  scenarioId: string
}

interface BayCell {
  year: number
  quarter: number
  bays: number
}

function bcKey(year: number, quarter: number) {
  return `${year}-Q${quarter}`
}

interface FactoryDraft {
  name: string
  bays: number
  matrix: Record<string, BayCell>
}

function draftFromFactory(f: Factory): FactoryDraft {
  const matrix: Record<string, BayCell> = {}
  for (const bc of f.bay_counts ?? []) {
    matrix[bcKey(bc.year, bc.quarter)] = {
      year: bc.year,
      quarter: bc.quarter,
      bays: bc.bays,
    }
  }
  return { name: f.name, bays: f.bays, matrix }
}

export function FactoryEditor({ scenarioId }: Props) {
  const [factories, setFactories] = useState<Factory[]>([])
  const [error, setError] = useState<string | null>(null)
  const [expanded, setExpanded] = useState<Record<string, boolean>>({})
  const [drafts, setDrafts] = useState<Record<string, FactoryDraft>>({})
  const [newName, setNewName] = useState('')
  const [newBays, setNewBays] = useState(10)
  const [yearRange, setYearRange] = useState<{ from: number; to: number }>(() => {
    const y = new Date().getFullYear()
    return { from: y, to: y + 1 }
  })

  async function reload() {
    try {
      setError(null)
      const list = await api.listFactories(scenarioId)
      setFactories(list)
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'load failed')
    }
  }
  useEffect(() => {
    if (scenarioId) void reload()
  }, [scenarioId])

  async function handleCreate() {
    if (!newName.trim()) return
    try {
      await api.createFactory(scenarioId, newName.trim(), newBays, [])
      setNewName('')
      setNewBays(10)
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'create failed')
    }
  }

  async function handleSave(f: Factory) {
    const d = drafts[f.id] ?? draftFromFactory(f)
    const bay_counts = Object.values(d.matrix).filter((c) => c.bays >= 0)
    try {
      await api.updateFactory(f.id, d.name.trim(), d.bays, bay_counts)
      setDrafts((s) => {
        const x = { ...s }
        delete x[f.id]
        return x
      })
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'save failed')
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete factory?')) return
    try {
      await api.deleteFactory(id)
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'delete failed')
    }
  }

  function draftFor(f: Factory): FactoryDraft {
    return drafts[f.id] ?? draftFromFactory(f)
  }
  function setDraft(id: string, fn: (d: FactoryDraft) => FactoryDraft) {
    setDrafts((s) => {
      const current = s[id] ?? draftFromFactory(factories.find((f) => f.id === id)!)
      return { ...s, [id]: fn(current) }
    })
  }

  const years: number[] = []
  for (let y = yearRange.from; y <= yearRange.to; y++) years.push(y)

  return (
    <div className="space-y-4">
      {error && <div className="text-sm text-rose-600">{error}</div>}

      <div className="flex items-center gap-3 text-sm">
        <span className="text-slate-600">Bay-count matrix years:</span>
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
        {factories.length === 0 && (
          <div className="px-3 py-6 text-center text-slate-500 text-sm">No factories yet.</div>
        )}
        {factories.map((f) => {
          const isOpen = expanded[f.id] ?? false
          const d = draftFor(f)
          const dirty =
            drafts[f.id] !== undefined &&
            JSON.stringify(drafts[f.id]) !== JSON.stringify(draftFromFactory(f))
          return (
            <div key={f.id} className="border-b border-slate-100 last:border-b-0">
              <div className="flex items-center gap-2 px-3 py-2">
                <button
                  className="text-slate-500 hover:text-slate-800"
                  onClick={() => setExpanded((e) => ({ ...e, [f.id]: !isOpen }))}
                  title={isOpen ? 'Hide per-quarter overrides' : 'Show per-quarter overrides'}
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
                  onChange={(e) => setDraft(f.id, (x) => ({ ...x, name: e.target.value }))}
                />
                <label className="text-xs text-slate-500">Base bays</label>
                <input
                  type="number"
                  min={0}
                  className="w-20 border border-slate-200 rounded px-2 py-1 text-center"
                  value={d.bays}
                  onChange={(e) =>
                    setDraft(f.id, (x) => ({ ...x, bays: parseInt(e.target.value) || 0 }))
                  }
                />
                {dirty && (
                  <button
                    className="text-emerald-700 hover:text-emerald-900"
                    onClick={() => handleSave(f)}
                    title="Save"
                  >
                    <Save className="w-4 h-4" />
                  </button>
                )}
                <button
                  className="text-rose-500 hover:text-rose-700"
                  onClick={() => handleDelete(f.id)}
                  title="Delete"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>
              {isOpen && (
                <div className="px-3 pb-4">
                  <div className="text-xs text-slate-500 mb-2">
                    Override bay count per quarter (leave blank to use the base value of{' '}
                    <span className="font-medium">{d.bays}</span>).
                  </div>
                  <div className="overflow-x-auto">
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
                              const k = bcKey(y, q)
                              const cell = d.matrix[k]
                              return (
                                <td key={q} className="px-1 py-1">
                                  <input
                                    type="number"
                                    min={0}
                                    placeholder={`(${d.bays})`}
                                    className="w-20 border border-slate-200 rounded px-2 py-1 text-center"
                                    value={cell?.bays ?? ''}
                                    onChange={(e) => {
                                      const v = e.target.value.trim()
                                      setDraft(f.id, (x) => {
                                        const next = { ...x, matrix: { ...x.matrix } }
                                        if (v === '') {
                                          delete next.matrix[k]
                                        } else {
                                          const n = parseInt(v)
                                          if (Number.isFinite(n) && n >= 0) {
                                            next.matrix[k] = {
                                              year: y,
                                              quarter: q,
                                              bays: n,
                                            }
                                          }
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
                </div>
              )}
            </div>
          )
        })}
        <div className="flex items-center gap-2 px-3 py-2 bg-slate-50">
          <input
            className="flex-1 border border-slate-300 rounded px-2 py-1 text-sm"
            placeholder="New factory name…"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <input
            type="number"
            min={0}
            className="w-20 border border-slate-300 rounded px-2 py-1 text-sm text-center"
            value={newBays}
            onChange={(e) => setNewBays(parseInt(e.target.value) || 0)}
            title="Base bays"
          />
          <button
            className="inline-flex items-center gap-1 px-3 py-1 bg-indigo-600 text-white text-sm rounded hover:bg-indigo-700"
            onClick={handleCreate}
          >
            <Plus className="w-4 h-4" />
            Add factory
          </button>
        </div>
      </div>
    </div>
  )
}
