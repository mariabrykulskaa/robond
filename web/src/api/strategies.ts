import api from "./client";

export interface StrategyInfo {
  id: string;
  name: string;
  description: string;
}

export interface RunResult {
  orders_count: number;
  message: string;
}

export async function listStrategies(): Promise<StrategyInfo[]> {
  const { data } = await api.get<StrategyInfo[]>("/strategies");
  return data;
}

export async function setStrategy(portfolioId: number, strategyName: string): Promise<void> {
  await api.put(`/portfolios/${portfolioId}/strategy`, { strategy_name: strategyName });
}

export async function clearStrategy(portfolioId: number): Promise<void> {
  await api.delete(`/portfolios/${portfolioId}/strategy`);
}

export async function runStrategy(portfolioId: number): Promise<RunResult> {
  const { data } = await api.post<RunResult>(`/portfolios/${portfolioId}/strategy/run`);
  return data;
}
