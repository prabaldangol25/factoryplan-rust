import axios from 'axios'
import type {
  Scenario,
  Factory,
  Product,
  Demand,
  PeriodType,
  SpreadMode,
  RunResult,
} from '../types'

const client = axios.create({
  baseURL: '/',
  headers: { 'Content-Type': 'application/json' },
})

export interface ApiError {
  status: number
  message: string
}

function rethrow(err: unknown): never {
  if (axios.isAxiosError(err)) {
    const status = err.response?.status ?? 0
    const msg =
      (err.response?.data as { error?: string } | undefined)?.error ??
      err.message ??
      'unknown error'
    const e: ApiError = { status, message: msg }
    throw e
  }
  throw err
}

// ---------- health ----------
export async function getHealth(): Promise<{ status: string; service: string; version: string }> {
  return client
    .get('/api/health')
    .then((r) => r.data)
    .catch(rethrow)
}

// ---------- scenarios ----------
export async function listScenarios(): Promise<Scenario[]> {
  return client.get('/api/scenarios').then((r) => r.data).catch(rethrow)
}

export async function createScenario(name: string, clone_from?: string): Promise<Scenario> {
  return client
    .post('/api/scenarios', { name, clone_from })
    .then((r) => r.data)
    .catch(rethrow)
}

export async function renameScenario(id: string, name: string): Promise<Scenario> {
  return client.put(`/api/scenarios/${id}`, { name }).then((r) => r.data).catch(rethrow)
}

export async function deleteScenario(id: string): Promise<void> {
  return client.delete(`/api/scenarios/${id}`).then(() => undefined).catch(rethrow)
}

export async function activateScenario(id: string): Promise<void> {
  return client.post(`/api/scenarios/${id}/activate`).then(() => undefined).catch(rethrow)
}

// ---------- factories ----------
export async function listFactories(scenarioId: string): Promise<Factory[]> {
  return client.get(`/api/scenarios/${scenarioId}/factories`).then((r) => r.data).catch(rethrow)
}

export async function createFactory(scenarioId: string, name: string, bays: number): Promise<Factory> {
  return client
    .post(`/api/scenarios/${scenarioId}/factories`, { name, bays })
    .then((r) => r.data)
    .catch(rethrow)
}

export async function updateFactory(id: string, name: string, bays: number): Promise<Factory> {
  return client.put(`/api/factories/${id}`, { name, bays }).then((r) => r.data).catch(rethrow)
}

export async function deleteFactory(id: string): Promise<void> {
  return client.delete(`/api/factories/${id}`).then(() => undefined).catch(rethrow)
}

// ---------- products ----------
export interface LeadTimeInput {
  year: number
  quarter: number
  lead_time_days: number
}

export async function listProducts(scenarioId: string): Promise<Product[]> {
  return client.get(`/api/scenarios/${scenarioId}/products`).then((r) => r.data).catch(rethrow)
}

export async function createProduct(
  scenarioId: string,
  name: string,
  lead_times: LeadTimeInput[],
): Promise<Product> {
  return client
    .post(`/api/scenarios/${scenarioId}/products`, { name, lead_times })
    .then((r) => r.data)
    .catch(rethrow)
}

export async function updateProduct(
  id: string,
  name: string,
  lead_times: LeadTimeInput[],
): Promise<Product> {
  return client.put(`/api/products/${id}`, { name, lead_times }).then((r) => r.data).catch(rethrow)
}

export async function deleteProduct(id: string): Promise<void> {
  return client.delete(`/api/products/${id}`).then(() => undefined).catch(rethrow)
}

// ---------- demand ----------
export interface DemandInput {
  product_id: string
  period_type: PeriodType
  year: number
  period_index: number
  quantity: number
  spread_mode: SpreadMode
}

export async function listDemand(scenarioId: string): Promise<Demand[]> {
  return client.get(`/api/scenarios/${scenarioId}/demand`).then((r) => r.data).catch(rethrow)
}

export async function createDemand(scenarioId: string, d: DemandInput): Promise<Demand> {
  return client
    .post(`/api/scenarios/${scenarioId}/demand`, d)
    .then((r) => r.data)
    .catch(rethrow)
}

export async function updateDemand(id: string, d: DemandInput): Promise<Demand> {
  return client.put(`/api/demand/${id}`, d).then((r) => r.data).catch(rethrow)
}

export async function deleteDemand(id: string): Promise<void> {
  return client.delete(`/api/demand/${id}`).then(() => undefined).catch(rethrow)
}

// ---------- run (Phase 2) ----------
export async function runScenario(scenarioId: string): Promise<RunResult> {
  return client.post(`/api/scenarios/${scenarioId}/run`).then((r) => r.data).catch(rethrow)
}

// ---------- import / export (Phase 5) ----------
export interface ImportResult {
  inserted: number
  skipped: number
  errors: string[]
}

export async function importDemandExcel(
  scenarioId: string,
  file: File,
): Promise<ImportResult> {
  const form = new FormData()
  form.append('file', file)
  return axios
    .post(`/api/scenarios/${scenarioId}/demand/import-excel`, form, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })
    .then((r) => r.data)
    .catch(rethrow)
}

export function exportRunCsvUrl(runId: string): string {
  return `/api/runs/${runId}/export.csv`
}

export function exportRunXlsxUrl(runId: string): string {
  return `/api/runs/${runId}/export.xlsx`
}
