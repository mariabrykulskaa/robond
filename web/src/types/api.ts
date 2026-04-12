export interface Portfolio {
  id: number;
  name: string;
  user_id: number;
  strategy_name: string | null;
  strategy_running: boolean | null;
  pending_strategy_run: boolean;
  created_at: string;
}

export interface PortfolioHolding {
  id: number;
  portfolio_id: number;
  isin: string;
  quantity: number;
  updated_at: string;
}

export interface PortfolioCash {
  id: number;
  portfolio_id: number;
  amount: string;
  currency: string;
  updated_at: string;
}

export interface PortfolioSnapshot {
  id: number;
  portfolio_id: number;
  date: string;
  market_value: string;
  cash: string;
  bonds_value: string;
}

export interface AuthResponse {
  access_token: string;
  refresh_token: string;
  user: UserInfo;
}

export interface UserInfo {
  id: number;
  email: string;
}
