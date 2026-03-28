use std::collections::HashMap;

use chrono::NaiveDate;
use financial::naive_date::xirr;
use rust_decimal::prelude::*;

use crate::{BondPersistentInfo, Isin, MarketOrder, MarketOrderType, Portfolio, Strategy};

fn calc_yield(current_bond_price: Decimal, current_date: NaiveDate, bond_persistent_info: &BondPersistentInfo) -> f64 {
    let mut cash_flow = vec![-current_bond_price.as_f64()];
    let mut dates = vec![current_date];
    for payment_info in bond_persistent_info.payments.iter() {
        cash_flow.push(payment_info.amount.as_f64());
        dates.push(payment_info.date);
    }

    xirr(&cash_flow, &dates, None).unwrap() // TODO: заменить unwrup() на более акуратную обработку ошибок
}

pub struct MostProfitableBondStrategy;

impl Strategy for MostProfitableBondStrategy {
    fn decide_trades(
        &self,
        current_date: NaiveDate,
        portfolio: &Portfolio,
        bonds_info: &HashMap<Isin, BondPersistentInfo>,
        bonds_prices: &HashMap<Isin, Decimal>,
    ) -> Vec<MarketOrder> {
        let mut isin_to_yield = HashMap::<Isin, f64>::new();
        for (isin, bond_persistent_info) in bonds_info {
            let bond_yield = calc_yield(*bonds_prices.get(isin).unwrap(), current_date, bond_persistent_info);
            isin_to_yield.insert(isin.clone(), bond_yield);
        }

        let portfolio_market_price = portfolio.market_price(bonds_prices);
        let mut orders = Vec::<MarketOrder>::new();
        for (isin, &count) in portfolio.bonds_count.iter() {
            orders.push(MarketOrder {
                isin: isin.clone(),
                order_type: MarketOrderType::Sell,
                count,
            });
        }

        let most_profitable_bond = isin_to_yield
            .iter()
            .max_by(|&(_key1, val1), &(_key2, val2)| val1.partial_cmp(val2).unwrap())
            .map(|(isin, _bond_yield)| isin);
        match most_profitable_bond {
            None => {}
            Some(most_profitable_bond) => {
                let count = (portfolio_market_price / bonds_prices.get(most_profitable_bond).unwrap())
                    .to_i64()
                    .unwrap();
                orders.push(MarketOrder {
                    isin: most_profitable_bond.clone(),
                    order_type: MarketOrderType::Buy,
                    count,
                });
            }
        }

        orders
    }
}
