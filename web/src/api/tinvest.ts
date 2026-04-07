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
  endpoint: string
): Promise<AccountInfo[]> {
  const { data } = await api.post<AccountInfo[]>("/tinvest/accounts", {
    token,
    endpoint,
  });
  return data;
}

export async function getStatus(): Promise<TInvestStatus> {
  const { data } = await api.get<TInvestStatus>("/tinvest/status");
  return data;
}

export async function connect(
  token: string,
  accountId: string,
  endpoint: string
): Promise<TInvestStatus> {
  const { data } = await api.post<TInvestStatus>("/tinvest/connect", {
    token,
    account_id: accountId,
    endpoint,
  });
  return data;
}

export async function disconnect(): Promise<TInvestStatus> {
  const { data } = await api.delete<TInvestStatus>("/tinvest/disconnect");
  return data;
}

export async function importPortfolio(portfolioId: number): Promise<ImportResult> {
  const { data } = await api.post<ImportResult>(`/tinvest/import/${portfolioId}`);
  return data;
}
