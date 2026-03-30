use std::collections::HashMap;

use chrono::{Duration, NaiveDate};
use financial::naive_date::xirr;
use rust_decimal::prelude::*;

use crate::{BondPersistentInfo, Isin, MarketOrder, MarketOrderType, Portfolio, Strategy};

fn calc_yield(current_bond_price: Decimal, current_date: NaiveDate, bond_persistent_info: &BondPersistentInfo) -> f64 {
    let mut cash_flow = vec![-current_bond_price.as_f64()];
    let mut dates = vec![current_date];
    for payment_info in bond_persistent_info.payments.iter() {
        if payment_info.date > current_date + Duration::days(5) {
            cash_flow.push(payment_info.amount.as_f64());
            dates.push(payment_info.date);
        }
    }

    xirr(&cash_flow, &dates, None).unwrap_or(-1.)
}

pub struct MostProfitableBondStrategy;

impl Strategy for MostProfitableBondStrategy {
    fn decide_trades(
        &self,
        current_date: NaiveDate,
        portfolio: &Portfolio,
        bonds_info: &HashMap<Isin, BondPersistentInfo>,
        bonds_prices: &HashMap<Isin, Decimal>,
        bonds_volumes: &HashMap<Isin, i64>,
    ) -> Vec<MarketOrder> {
        let mut isin_to_yield = HashMap::<Isin, f64>::new();
        for (isin, bond_persistent_info) in bonds_info {
            let current_bond_price = match bonds_prices.get(isin) {
                None => continue,
                Some(current_bond_price) => *current_bond_price,
            };
            if current_bond_price > Decimal::from(0) {
                let bond_yield = calc_yield(current_bond_price, current_date, bond_persistent_info);
                isin_to_yield.insert(isin.clone(), bond_yield);
            }
        }

        let portfolio_market_price = portfolio.market_price(bonds_prices);
        let mut orders = Vec::<MarketOrder>::new();
        for (isin, &count) in portfolio.bonds_count.iter() {
            if count > 0 && bonds_prices.contains_key(isin) {
                orders.push(MarketOrder {
                    isin: isin.clone(),
                    order_type: MarketOrderType::Sell,
                    count,
                });
            }
        }

        let most_profitable_bond = isin_to_yield
            .iter()
            .max_by(|&(_key1, val1), &(_key2, val2)| val1.partial_cmp(val2).unwrap());
        match most_profitable_bond {
            None => {}
            Some((most_profitable_bond, &bond_yield)) => {
                if bond_yield > 0. {
                    let mut count = (portfolio_market_price / bonds_prices.get(most_profitable_bond).unwrap())
                        .to_i64()
                        .unwrap();

                    // Ограничиваем по дневному объёму торгов
                    if let Some(&day_vol) = bonds_volumes.get(most_profitable_bond)
                        && day_vol > 0
                        && count > day_vol
                    {
                        count = day_vol;
                    }

                    // Ограничиваем по объёму выпуска
                    if let Some(info) = bonds_info.get(most_profitable_bond)
                        && let Some(issue_vol) = info.bond_info.issue_volume
                        && issue_vol > 0
                        && count > issue_vol
                    {
                        count = issue_vol;
                    }

                    if count > 0 {
                        orders.push(MarketOrder {
                            isin: most_profitable_bond.clone(),
                            order_type: MarketOrderType::Buy,
                            count,
                        });
                    }
                }
            }
        }

        orders
    }
}
