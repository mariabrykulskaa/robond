import api from "./client";
import type { Portfolio, PortfolioHolding, PortfolioSnapshot } from "../types/api";

export async function listPortfolios(): Promise<Portfolio[]> {
  const { data } = await api.get<Portfolio[]>("/portfolios");
  return data;
}

export async function createPortfolio(name: string): Promise<Portfolio> {
  const { data } = await api.post<Portfolio>("/portfolios", { name });
  return data;
}

export async function getPortfolio(id: number): Promise<Portfolio> {
  const { data } = await api.get<Portfolio>(`/portfolios/${id}`);
  return data;
}

export async function getHoldings(portfolioId: number): Promise<PortfolioHolding[]> {
  const { data } = await api.get<PortfolioHolding[]>(`/portfolios/${portfolioId}/holdings`);
  return data;
}

export async function getCash(portfolioId: number): Promise<{ amount: string }> {
  const { data } = await api.get<{ amount: string }>(`/portfolios/${portfolioId}/cash`);
  return data;
}

export async function getSnapshots(portfolioId: number): Promise<PortfolioSnapshot[]> {
  const { data } = await api.get<PortfolioSnapshot[]>(`/portfolios/${portfolioId}/snapshots`);
  return data;
}

export async function getTotalReturn(portfolioId: number): Promise<{ total_return: string | null }> {
  const { data } = await api.get<{ total_return: string | null }>(`/portfolios/${portfolioId}/return`);
  return data;
}

export interface HoldingValue {
  isin: string;
  name: string;
  quantity: number;
  price: string;
  value: string;
  estimated: boolean;
}

export interface PortfolioValue {
  holdings: HoldingValue[];
  bonds_value: string;
  cash: string;
  total_value: string;
}

export async function getPortfolioValue(portfolioId: number): Promise<PortfolioValue> {
  const { data } = await api.get<PortfolioValue>(`/portfolios/${portfolioId}/value`);
  return data;
}
