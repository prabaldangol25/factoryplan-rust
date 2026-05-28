import { useEffect, useState } from 'react'
import { Plus, Trash2, Save } from 'lucide-react'
import type { Factory } from '../types'
import * as api from '../api'

interface Props {
  scenarioId: string
}

export function FactoryEditor({ scenarioId }: Props) {
  const [factories, setFactories] = useState<Factory[]>([])
  const [error, setError] = useState<string | null>(null)
  const [newName, setNewName] = useState('')
  const [newBays, setNewBays] = useState(10)
  const [drafts, setDrafts] = useState<Record<string, { name: string; bays: number }>>({})

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
      await api.createFactory(scenarioId, newName.trim(), newBays)
      setNewName('')
      setNewBays(10)
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'create failed')
    }
  }

  async function handleSave(id: string) {
    const d = drafts[id]
    if (!d) return
    try {
      await api.updateFactory(id, d.name.trim(), d.bays)
      setDrafts((s) => {
        const x = { ...s }
        delete x[id]
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

  function draftFor(f: Factory) {
    return drafts[f.id] ?? { name: f.name, bays: f.bays }
  }
  function setDraft(id: string, patch: Partial<{ name: string; bays: number }>) {
    setDrafts((s) => ({
      ...s,
      [id]: { ...(s[id] ?? factoryDefault(id)), ...patch },
    }))
  }
  function factoryDefault(id: string) {
    const f = factories.find((x) => x.id === id)
    return f ? { name: f.name, bays: f.bays } : { name: '', bays: 0 }
  }

  return (
    <div className="space-y-4">
      {error && <div className="text-sm text-rose-600">{error}</div>}

      <div className="rounded-lg border border-slate-200 bg-white overflow-hidden">
        <table className="min-w-full text-sm">
          <thead className="bg-slate-50 text-slate-600">
            <tr>
              <th className="text-left px-3 py-2 font-medium">Name</th>
              <th className="text-left px-3 py-2 font-medium w-32">Bays</th>
              <th className="w-24"></th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {factories.length === 0 && (
              <tr>
                <td colSpan={3} className="px-3 py-6 text-center text-slate-500">
                  No factories yet.
                </td>
              </tr>
            )}
            {factories.map((f) => {
              const d = draftFor(f)
              const dirty = drafts[f.id] !== undefined && (d.name !== f.name || d.bays !== f.bays)
              return (
                <tr key={f.id}>
                  <td className="px-3 py-2">
                    <input
                      className="w-full border border-slate-200 rounded px-2 py-1"
                      value={d.name}
                      onChange={(e) => setDraft(f.id, { name: e.target.value })}
                    />
                  </td>
                  <td className="px-3 py-2">
                    <input
                      type="number"
                      min={0}
                      className="w-24 border border-slate-200 rounded px-2 py-1"
                      value={d.bays}
                      onChange={(e) => setDraft(f.id, { bays: parseInt(e.target.value) || 0 })}
                    />
                  </td>
                  <td className="px-3 py-2 text-right">
                    {dirty && (
                      <button
                        className="inline-flex items-center gap-1 text-emerald-700 hover:text-emerald-900 mr-2"
                        onClick={() => handleSave(f.id)}
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
                  </td>
                </tr>
              )
            })}
          </tbody>
          <tfoot className="bg-slate-50">
            <tr>
              <td className="px-3 py-2">
                <input
                  className="w-full border border-slate-300 rounded px-2 py-1"
                  placeholder="New factory name…"
                  value={newName}
                  onChange={(e) => setNewName(e.target.value)}
                />
              </td>
              <td className="px-3 py-2">
                <input
                  type="number"
                  min={0}
                  className="w-24 border border-slate-300 rounded px-2 py-1"
                  value={newBays}
                  onChange={(e) => setNewBays(parseInt(e.target.value) || 0)}
                />
              </td>
              <td className="px-3 py-2 text-right">
                <button
                  className="inline-flex items-center gap-1 px-3 py-1 bg-indigo-600 text-white text-sm rounded hover:bg-indigo-700"
                  onClick={handleCreate}
                >
                  <Plus className="w-4 h-4" />
                  Add
                </button>
              </td>
            </tr>
          </tfoot>
        </table>
      </div>
    </div>
  )
}
