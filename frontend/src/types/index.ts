export interface Scenario {
  id: string
  name: string
  created_at: string
  updated_at: string
  is_active: boolean
}

export interface BayCountRow {
  id: string
  factory_id: string
  year: number
  quarter: number
  bays: number
}

export interface Factory {
  id: string
  scenario_id: string
  name: string
  bays: number
  bay_counts: BayCountRow[]
}

export interface LeadTimeRow {
  id: string
  product_id: string
  year: number
  quarter: number
  lead_time_days: number
}

export interface Product {
  id: string
  scenario_id: string
  name: string
  lead_times: LeadTimeRow[]
}

export type PeriodType = 'month' | 'quarter'
export type SpreadMode = 'even' | 'start' | 'end'

export interface Demand {
  id: string
  scenario_id: string
  product_id: string
  period_type: PeriodType
  year: number
  period_index: number
  quantity: number
  spread_mode: SpreadMode
}

export interface ScheduleRun {
  id: string
  scenario_id: string
  run_at: string
  total_demand: number
  shipped_on_time: number
  unshippable: number
}

export interface ScheduledUnit {
  id: string
  run_id: string
  demand_id: string
  product_id: string
  factory_id: string | null
  bay_index: number | null
  required_start: string
  due_date: string
  status: 'shipped' | 'unshippable'
}

export interface PerProductLtTarget {
  product_id: string
  product_name: string
  current_lead_time_days: number
  target_lead_time_days: number
}

export interface BaysNeededRec {
  bays_to_add: number
  suggested_factory_id: string | null
  suggested_factory_name: string | null
}

export interface UniformLtPctRec {
  reduction_pct: number
}

export interface Recommendation {
  bays_needed: BaysNeededRec | null
  uniform_lt_pct: UniformLtPctRec | null
  per_product_lt: PerProductLtTarget[]
}

export interface RunResult {
  run: ScheduleRun
  units: ScheduledUnit[]
  recommendation: Recommendation
}
