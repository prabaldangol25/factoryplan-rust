import { useEffect, useMemo, useRef, useState } from 'react'
import { Plus, Trash2, Upload, Pencil, Check, X } from 'lucide-react'
import type { Demand, PeriodType, Product, SerialMode, SpreadMode } from '../types'
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
  serial_mode: SerialMode
  serial_start: string
  serial_list: string
}

/** Increment the trailing number of `start` by `k`, preserving prefix + width. */
function serialAt(start: string, k: number): string {
  const m = start.match(/^(.*?)(\d+)$/)
  if (!m) return k === 0 ? start : `${start}-${k}`
  const [, prefix, digits] = m
  const num = BigInt(digits) + BigInt(k)
  return prefix + num.toString().padStart(digits.length, '0')
}

/** Split pasted text into one serial per line, dropping trailing blanks. */
function parseSerialLines(text: string): string[] {
  const lines = text.replace(/\r/g, '\n').split('\n').map((s) => s.trim())
  while (lines.length && lines[lines.length - 1] === '') lines.pop()
  return lines
}

/** Short human summary of a demand row's serials, for the table. */
function serialSummary(d: Demand, quantity: number): string {
  if (d.serial_mode === 'sequence' && d.serial_start) {
    const first = serialAt(d.serial_start, 0)
    if (quantity <= 1) return first
    return `${first} … ${serialAt(d.serial_start, quantity - 1)}`
  }
  if (d.serial_mode === 'list') {
    const n = d.serial_list ? parseSerialLines(d.serial_list).length : 0
    return `List (${n})`
  }
  return '—'
}

const MONTH_NAMES = [
  'Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
  'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec',
]

export function DemandEditor({ scenarioId }: Props) {
  const [demand, setDemand] = useState<Demand[]>([])
  const [products, setProducts] = useState<Product[]>([])
  const [error, setError] = useState<string | null>(null)
  const [importResult, setImportResult] = useState<{ inserted: number; skipped: number; errors: string[] } | null>(null)
  const fileRef = useRef<HTMLInputElement>(null)

  function emptyForm(): NewDemandForm {
    return {
      product_id: '',
      period_type: 'quarter',
      year: new Date().getFullYear(),
      period_index: 1,
      quantity: 10,
      spread_mode: 'even',
      serial_mode: 'none',
      serial_start: '',
      serial_list: '',
    }
  }

  const [form, setForm] = useState<NewDemandForm>(emptyForm)
  /** id of the demand row currently being edited (null = creating a new row). */
  const [editingId, setEditingId] = useState<string | null>(null)
  const formRef = useRef<HTMLDivElement>(null)

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

  function formToInput(f: NewDemandForm): api.DemandInput {
    return {
      product_id: f.product_id,
      period_type: f.period_type,
      year: f.year,
      period_index: f.period_index,
      quantity: f.quantity,
      spread_mode: f.spread_mode,
      serial_mode: f.serial_mode,
      serial_start: f.serial_mode === 'sequence' ? f.serial_start : null,
      serial_list: f.serial_mode === 'list' ? f.serial_list : null,
    }
  }

  function startEdit(d: Demand) {
    setEditingId(d.id)
    setError(null)
    setForm({
      product_id: d.product_id,
      period_type: d.period_type,
      year: d.year,
      period_index: d.period_index,
      quantity: d.quantity,
      spread_mode: d.spread_mode,
      serial_mode: d.serial_mode,
      serial_start: d.serial_start ?? '',
      serial_list: d.serial_list ?? '',
    })
    // Scroll the form into view so the user sees the populated editor.
    requestAnimationFrame(() => formRef.current?.scrollIntoView({ behavior: 'smooth', block: 'nearest' }))
  }

  function cancelEdit() {
    setEditingId(null)
    setError(null)
    setForm((f) => ({ ...emptyForm(), product_id: f.product_id }))
  }

  async function handleSubmit() {
    if (!form.product_id) {
      setError('select a product first')
      return
    }
    try {
      if (editingId) {
        await api.updateDemand(editingId, formToInput(form))
        setEditingId(null)
        setForm((f) => ({ ...emptyForm(), product_id: f.product_id }))
      } else {
        await api.createDemand(scenarioId, formToInput(form))
      }
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? (editingId ? 'update failed' : 'create failed'))
    }
  }

  async function handleImport(file: File) {
    try {
      setError(null)
      setImportResult(null)
      const r = await api.importDemandExcel(scenarioId, file)
      setImportResult(r)
      await reload()
    } catch (e: unknown) {
      setError(((e as { message?: string }).message) ?? 'import failed')
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete demand row?')) return
    try {
      await api.deleteDemand(id)
      if (editingId === id) cancelEdit()
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

      <div className="flex items-center justify-between gap-3 flex-wrap">
        <div className="text-sm text-slate-600">
          {demand.length} row{demand.length !== 1 && 's'} ·{' '}
          {demand.reduce((s, d) => s + d.quantity, 0)} units
        </div>
        <div>
          <input
            ref={fileRef}
            type="file"
            accept=".xlsx,.xls"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0]
              if (f) void handleImport(f)
              if (fileRef.current) fileRef.current.value = ''
            }}
          />
          <button
            className="inline-flex items-center gap-1 px-3 py-1.5 border border-slate-300 bg-white text-slate-700 text-sm rounded hover:bg-slate-50"
            onClick={() => fileRef.current?.click()}
          >
            <Upload className="w-4 h-4" />
            Import Excel
          </button>
        </div>
      </div>

      {importResult && (
        <div className="rounded-lg border border-slate-200 bg-slate-50 p-3 text-sm">
          <div className="font-medium">
            Imported {importResult.inserted} row{importResult.inserted !== 1 && 's'},{' '}
            skipped {importResult.skipped}.
          </div>
          {importResult.errors.length > 0 && (
            <ul className="mt-2 list-disc list-inside text-rose-700 text-xs space-y-0.5">
              {importResult.errors.slice(0, 10).map((e, i) => (
                <li key={i}>{e}</li>
              ))}
              {importResult.errors.length > 10 && (
                <li>…and {importResult.errors.length - 10} more</li>
              )}
            </ul>
          )}
        </div>
      )}

      <div className="rounded-lg border border-slate-200 bg-white overflow-hidden">
        <table className="min-w-full text-sm">
          <thead className="bg-slate-50 text-slate-600">
            <tr>
              <th className="text-left px-3 py-2 font-medium">Product</th>
              <th className="text-left px-3 py-2 font-medium">Period</th>
              <th className="text-left px-3 py-2 font-medium w-24">Quantity</th>
              <th className="text-left px-3 py-2 font-medium w-28">Spread</th>
              <th className="text-left px-3 py-2 font-medium">Serials</th>
              <th className="w-12"></th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-100">
            {demand.length === 0 && (
              <tr>
                <td colSpan={6} className="px-3 py-6 text-center text-slate-500">
                  No demand rows yet.
                </td>
              </tr>
            )}
            {demand.map((d) => (
              <tr key={d.id} className={editingId === d.id ? 'bg-indigo-50' : undefined}>
                <td className="px-3 py-2">{productMap.get(d.product_id)?.name ?? '(unknown)'}</td>
                <td className="px-3 py-2">{periodLabel(d)}</td>
                <td className="px-3 py-2">{d.quantity}</td>
                <td className="px-3 py-2 text-slate-600">{d.spread_mode}</td>
                <td className="px-3 py-2 text-slate-600 font-mono text-xs">
                  {serialSummary(d, d.quantity)}
                </td>
                <td className="px-3 py-2 text-right whitespace-nowrap">
                  <div className="inline-flex items-center gap-2">
                    <button
                      className="text-indigo-500 hover:text-indigo-700"
                      onClick={() => startEdit(d)}
                      title="Edit"
                    >
                      <Pencil className="w-4 h-4" />
                    </button>
                    <button
                      className="text-rose-500 hover:text-rose-700"
                      onClick={() => handleDelete(d.id)}
                      title="Delete"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        {/* New / edit demand form */}
        <div
          ref={formRef}
          className={
            'border-t px-3 py-3 space-y-3 ' +
            (editingId ? 'border-indigo-200 bg-indigo-50/60' : 'border-slate-200 bg-slate-50')
          }
        >
          <div className="flex items-center justify-between">
            <div className="text-xs font-medium uppercase tracking-wide text-slate-500">
              {editingId ? 'Edit demand row' : 'Add demand row'}
            </div>
            {editingId && (
              <button
                className="inline-flex items-center gap-1 text-xs text-slate-500 hover:text-slate-800"
                onClick={cancelEdit}
              >
                <X className="w-3.5 h-3.5" />
                Cancel edit
              </button>
            )}
          </div>
          <div className="flex items-end gap-3 flex-wrap">
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
          </div>
          {/* Serials row */}
          <div className="flex items-end gap-3 flex-wrap">
            <div>
              <label className="block text-xs text-slate-500 mb-1">Serials</label>
              <select
                className="border border-slate-300 rounded px-2 py-1 text-sm bg-white"
                value={form.serial_mode}
                onChange={(e) =>
                  setForm((f) => ({ ...f, serial_mode: e.target.value as SerialMode }))
                }
              >
                <option value="none">None</option>
                <option value="sequence">Leading serial (auto-increment)</option>
                <option value="list">Paste list</option>
              </select>
            </div>

            {form.serial_mode === 'sequence' && (
              <div>
                <label className="block text-xs text-slate-500 mb-1">Start serial</label>
                <input
                  className="w-40 border border-slate-300 rounded px-2 py-1 text-sm font-mono"
                  placeholder="e.g. WID-0001"
                  value={form.serial_start}
                  onChange={(e) => setForm((f) => ({ ...f, serial_start: e.target.value }))}
                />
                {form.serial_start && (
                  <div className="text-xs text-slate-500 mt-1 font-mono">
                    {form.quantity <= 1
                      ? serialAt(form.serial_start, 0)
                      : `${serialAt(form.serial_start, 0)} … ${serialAt(
                          form.serial_start,
                          form.quantity - 1,
                        )}`}
                  </div>
                )}
              </div>
            )}

            {form.serial_mode === 'list' && (
              <div className="flex-1 min-w-[18rem]">
                <label className="block text-xs text-slate-500 mb-1">
                  Paste serials (one per line — copy a column from Excel)
                </label>
                <textarea
                  className="w-full h-20 border border-slate-300 rounded px-2 py-1 text-sm font-mono resize-y"
                  placeholder={'SN-001\nSN-002\nSN-003'}
                  value={form.serial_list}
                  onChange={(e) => setForm((f) => ({ ...f, serial_list: e.target.value }))}
                />
                {(() => {
                  const n = parseSerialLines(form.serial_list).length
                  const mismatch = n !== form.quantity
                  return (
                    <div
                      className={`text-xs mt-1 ${mismatch ? 'text-amber-600' : 'text-slate-500'}`}
                    >
                      {n} serial{n !== 1 && 's'} for {form.quantity} unit
                      {form.quantity !== 1 && 's'}
                      {mismatch &&
                        (n < form.quantity
                          ? ' — extra units will have no serial'
                          : ' — extra serials will be ignored')}
                    </div>
                  )
                })()}
              </div>
            )}

            <div className="flex items-center gap-2">
              <button
                className="inline-flex items-center gap-1 px-3 py-1.5 bg-indigo-600 text-white text-sm rounded hover:bg-indigo-700"
                onClick={handleSubmit}
              >
                {editingId ? <Check className="w-4 h-4" /> : <Plus className="w-4 h-4" />}
                {editingId ? 'Save changes' : 'Add'}
              </button>
              {editingId && (
                <button
                  className="inline-flex items-center gap-1 px-3 py-1.5 border border-slate-300 bg-white text-slate-700 text-sm rounded hover:bg-slate-50"
                  onClick={cancelEdit}
                >
                  Cancel
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
