import api from "./client";

export interface BondInfo {
  name: string;
  ticker: string;
  isin: string;
  figi: string;
  currency: string;
  nominal: string | null;
  aci_value: string | null;
  coupon_quantity_per_year: number;
  maturity_date: string | null;
  placement_date: string | null;
  country_of_risk_name: string;
  sector: string;
  lot: number;
  exchange: string;
  short_enabled: boolean;
  buy_available: boolean;
  sell_available: boolean;
}

export async function getBondInfo(isin: string): Promise<BondInfo> {
  const { data } = await api.get<BondInfo>(`/bonds/${isin}`);
  return data;
}
