import { useEffect, useState } from 'react'
import {
  Factory as FactoryIcon,
  Building2,
  Package,
  ListChecks,
  Play,
  BarChart3,
} from 'lucide-react'
import * as api from './api'
import type { Scenario } from './types'
import { ScenarioSwitcher } from './components/ScenarioSwitcher'
import { FactoryEditor } from './components/FactoryEditor'
import { ProductEditor } from './components/ProductEditor'
import { DemandEditor } from './components/DemandEditor'
import './App.css'

type Tab = 'factories' | 'products' | 'demand' | 'run' | 'results'

function App() {
  const [scenarios, setScenarios] = useState<Scenario[]>([])
  const [activeId, setActiveId] = useState<string | null>(null)
  const [tab, setTab] = useState<Tab>('factories')
  const [bootError, setBootError] = useState<string | null>(null)

  async function reloadScenarios() {
    try {
      const list = await api.listScenarios()
      setScenarios(list)
      if (!activeId && list.length > 0) {
        setActiveId(list[0].id)
      }
      if (list.length === 0 && activeId) setActiveId(null)
    } catch (e: unknown) {
      setBootError(((e as { message?: string }).message) ?? 'failed to load scenarios')
    }
  }

  useEffect(() => {
    void reloadScenarios()
  }, [])

  if (bootError) {
    return (
      <div className="min-h-full flex items-center justify-center p-6">
        <div className="max-w-md text-center text-rose-700">
          <div className="font-semibold mb-2">Backend not reachable</div>
          <div className="text-sm">{bootError}</div>
          <div className="text-sm mt-3 text-slate-500">
            Run <code className="bg-slate-100 px-1.5 py-0.5 rounded">cargo run</code> in the
            backend/ directory.
          </div>
        </div>
      </div>
    )
  }

  const tabs: Array<{ key: Tab; label: string; icon: typeof FactoryIcon }> = [
    { key: 'factories', label: 'Factories', icon: Building2 },
    { key: 'products', label: 'Products', icon: Package },
    { key: 'demand', label: 'Demand', icon: ListChecks },
    { key: 'run', label: 'Run', icon: Play },
    { key: 'results', label: 'Results', icon: BarChart3 },
  ]

  return (
    <div className="min-h-full bg-slate-50 text-slate-900">
      <header className="border-b border-slate-200 bg-white">
        <div className="max-w-6xl mx-auto px-6 py-3 flex items-center gap-3">
          <FactoryIcon className="w-6 h-6 text-indigo-600" />
          <h1 className="text-lg font-semibold">factoryplan-rust</h1>
          <span className="ml-auto text-xs text-slate-500">Phase 1 · CRUD</span>
        </div>
      </header>

      <ScenarioSwitcher
        scenarios={scenarios}
        activeId={activeId}
        onChange={setActiveId}
        onReload={reloadScenarios}
      />

      <nav className="border-b border-slate-200 bg-white">
        <div className="max-w-6xl mx-auto px-6 flex gap-1">
          {tabs.map((t) => {
            const Icon = t.icon
            const active = tab === t.key
            return (
              <button
                key={t.key}
                onClick={() => setTab(t.key)}
                className={
                  'flex items-center gap-2 px-3 py-2 text-sm border-b-2 ' +
                  (active
                    ? 'border-indigo-600 text-indigo-700 font-medium'
                    : 'border-transparent text-slate-600 hover:text-slate-900')
                }
              >
                <Icon className="w-4 h-4" />
                {t.label}
              </button>
            )
          })}
        </div>
      </nav>

      <main className="max-w-6xl mx-auto px-6 py-6">
        {activeId == null ? (
          <div className="rounded-lg border border-dashed border-slate-300 p-10 text-center text-slate-500">
            Create a scenario to get started.
          </div>
        ) : (
          <>
            {tab === 'factories' && <FactoryEditor scenarioId={activeId} />}
            {tab === 'products' && <ProductEditor scenarioId={activeId} />}
            {tab === 'demand' && <DemandEditor scenarioId={activeId} />}
            {tab === 'run' && (
              <div className="rounded-lg border border-dashed border-slate-300 p-10 text-center text-slate-500">
                Run will be implemented in Phase 2.
              </div>
            )}
            {tab === 'results' && (
              <div className="rounded-lg border border-dashed border-slate-300 p-10 text-center text-slate-500">
                Results views land in Phase 4.
              </div>
            )}
          </>
        )}
      </main>
    </div>
  )
}

export default App
