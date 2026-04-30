import api from "./client";

export interface BondInfo {
  name: string;
  ticker: string;
  isin: string;
  currency: string | null;
  nominal: string | null;
  aci_value: string | null;
  coupon_quantity_per_year: number | null;
  maturity_date: string | null;
  country_of_risk_name: string | null;
  sector: string | null;
  exchange: string | null;
  coupon_type: string | null;
  coupon_amount: string | null;
  next_coupon_date: string | null;
  floating_coupon: boolean;
  amortization: boolean;
  perpetual: boolean;
  buy_available: boolean;
  sell_available: boolean;
}

export async function getBondInfo(isin: string, portfolioId: number): Promise<BondInfo> {
  const { data } = await api.get<BondInfo>(`/bonds/${isin}`, {
    params: { portfolio_id: portfolioId },
  });
  return data;
}
