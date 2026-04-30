import api from "./client";

export interface TInvestStatus {
  connected: boolean;
  account_id: string | null;
  endpoint: string | null;
}

export interface AccountInfo {
  id: string;
  name: string;
  account_type: string;
}

export interface ImportResult {
  holdings_imported: number;
  cash_rub: string;
}

export async function fetchAccounts(
  token: string,
  endpoint: string,
  initialAmount?: number
): Promise<AccountInfo[]> {
  const { data } = await api.post<AccountInfo[]>("/tinvest/accounts", {
    token,
    endpoint,
    initial_amount: initialAmount || undefined,
  });
  return data;
}

export async function getStatus(portfolioId: number): Promise<TInvestStatus> {
  const { data } = await api.get<TInvestStatus>(`/portfolios/${portfolioId}/tinvest-status`);
  return data;
}

export async function connect(
  portfolioId: number,
  token: string,
  accountId: string,
  endpoint: string
): Promise<TInvestStatus> {
  const { data } = await api.post<TInvestStatus>(`/portfolios/${portfolioId}/connect`, {
    token,
    account_id: accountId,
    endpoint,
  });
  return data;
}

export async function disconnect(portfolioId: number): Promise<TInvestStatus> {
  const { data } = await api.delete<TInvestStatus>(`/portfolios/${portfolioId}/disconnect`);
  return data;
}

export async function importPortfolio(portfolioId: number): Promise<ImportResult> {
  const { data } = await api.post<ImportResult>(`/portfolios/${portfolioId}/import`);
  return data;
}
