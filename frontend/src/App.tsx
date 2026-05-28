import { useEffect, useState } from 'react'
import axios from 'axios'
import { Factory, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react'
import './App.css'

type HealthState =
  | { kind: 'loading' }
  | { kind: 'ok'; service: string; version: string }
  | { kind: 'error'; message: string }

function App() {
  const [health, setHealth] = useState<HealthState>({ kind: 'loading' })

  useEffect(() => {
    axios
      .get('/api/health')
      .then((res) => {
        setHealth({
          kind: 'ok',
          service: res.data.service,
          version: res.data.version,
        })
      })
      .catch((err) => {
        setHealth({
          kind: 'error',
          message: err?.message ?? 'unknown error',
        })
      })
  }, [])

  return (
    <div className="min-h-full bg-slate-50 text-slate-900">
      <header className="border-b border-slate-200 bg-white">
        <div className="max-w-5xl mx-auto px-6 py-4 flex items-center gap-3">
          <Factory className="w-6 h-6 text-indigo-600" />
          <h1 className="text-xl font-semibold">factoryplan-rust</h1>
          <span className="ml-auto text-xs text-slate-500">Phase 0 · scaffolding</span>
        </div>
      </header>

      <main className="max-w-5xl mx-auto px-6 py-10 space-y-6">
        <section className="rounded-lg border border-slate-200 bg-white p-6 shadow-sm">
          <h2 className="text-base font-semibold mb-3">Backend health</h2>
          {health.kind === 'loading' && (
            <div className="flex items-center gap-2 text-slate-600">
              <Loader2 className="w-4 h-4 animate-spin" />
              <span>Pinging backend…</span>
            </div>
          )}
          {health.kind === 'ok' && (
            <div className="flex items-center gap-2 text-emerald-700">
              <CheckCircle2 className="w-5 h-5" />
              <span>
                Backend reachable —{' '}
                <code className="text-sm bg-emerald-50 px-1.5 py-0.5 rounded">
                  {health.service} v{health.version}
                </code>
              </span>
            </div>
          )}
          {health.kind === 'error' && (
            <div className="flex items-start gap-2 text-rose-700">
              <AlertCircle className="w-5 h-5 mt-0.5" />
              <div>
                <div className="font-medium">Backend not reachable</div>
                <div className="text-sm text-rose-600 mt-1">
                  {health.message}. Start it with{' '}
                  <code className="bg-rose-50 px-1.5 py-0.5 rounded">cargo run</code> in
                  the <code className="bg-rose-50 px-1.5 py-0.5 rounded">backend/</code>{' '}
                  directory.
                </div>
              </div>
            </div>
          )}
        </section>

        <section className="rounded-lg border border-dashed border-slate-300 p-6 text-sm text-slate-500">
          Setup, Run, and Results views will be built in subsequent phases.
        </section>
      </main>
    </div>
  )
}

export default App
