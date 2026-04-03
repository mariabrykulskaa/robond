//! Преобразования между protobuf-типами `Quotation`/`MoneyValue` и `rust_decimal::Decimal`.
//!
//! Модуль доступен только при включённой feature `decimal`.

use rust_decimal::prelude::*;

use crate::proto::{MoneyValue, Quotation};

/// Преобразует `Quotation` в `Decimal`.
pub fn quotation_to_decimal(q: Quotation) -> Decimal {
    Decimal::from_i128_with_scale(q.units as i128 * 1_000_000_000 + q.nano as i128, 9)
}

/// Преобразует `Decimal` в `Quotation`.
///
/// Функция паникует, если целая часть не помещается в `i64`.
pub fn decimal_to_quotation(d: Decimal) -> Quotation {
    // Умножаем на 1e9, чтобы получить "нано-единицы".
    let nano_units = d * Decimal::from(1_000_000_000);

    // Берём целое значение (без дробной части).
    let total_nano = nano_units.to_i128().unwrap();

    let units = total_nano / 1_000_000_000;
    let units = i64::try_from(units).unwrap_or_else(|_| panic!("units value {units} does not fit in i64"));
    let nano = (total_nano % 1_000_000_000) as i32;

    Quotation { units, nano }
}

/// Преобразует `MoneyValue` в `Decimal` (валюта игнорируется).
pub fn money_value_to_decimal(m: &MoneyValue) -> Decimal {
    quotation_to_decimal(money_value_to_quotation(m))
}

/// Преобразует `Decimal` и валюту в `MoneyValue`.
///
/// Функция паникует, если целая часть не помещается в `i64`.
pub fn decimal_and_currency_to_money_value(d: Decimal, currency: &str) -> MoneyValue {
    let quotation = decimal_to_quotation(d);
    quotation_and_currency_to_money_value(&quotation, currency)
}

/// Преобразует `MoneyValue` в `Quotation`, отбрасывая валюту.
fn money_value_to_quotation(m: &MoneyValue) -> Quotation {
    Quotation {
        units: m.units,
        nano: m.nano,
    }
}

/// Преобразует `Quotation` и валюту в `MoneyValue`.
fn quotation_and_currency_to_money_value(q: &Quotation, currency: &str) -> MoneyValue {
    MoneyValue {
        currency: currency.into(),
        units: q.units,
        nano: q.nano,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cases() -> Vec<(Quotation, Decimal)> {
        vec![
            (
                Quotation {
                    units: 114,
                    nano: 250000000,
                },
                dec!(114.25),
            ),
            (
                Quotation {
                    units: -200,
                    nano: -200000000,
                },
                dec!(-200.20),
            ),
            (
                Quotation {
                    units: -0,
                    nano: -10000000,
                },
                dec!(-0.01),
            ),
        ]
    }

    #[test]
    fn test_quotation_to_decimal() {
        for (quotation, decimal) in cases() {
            assert_eq!(quotation_to_decimal(quotation), decimal);
        }
    }

    #[test]
    fn test_decimal_to_quotation() {
        for (quotation, decimal) in cases() {
            assert_eq!(decimal_to_quotation(decimal), quotation);
        }
    }

    #[test]
    #[should_panic(expected = "Multiplication overflowed")]
    fn decimal_to_quotation_overflow() {
        decimal_to_quotation(Decimal::MAX);
    }

    #[test]
    #[should_panic(expected = "does not fit in i64")]
    fn decimal_to_quotation_overflow2() {
        decimal_to_quotation(Decimal::MAX / dec!(1_000_000_000));
    }

    #[test]
    fn test_money_value_to_quotation() {
        let money = MoneyValue {
            currency: "RUB".to_string(),
            units: 123,
            nano: 450_000_000,
        };

        assert_eq!(
            money_value_to_quotation(&money),
            Quotation {
                units: 123,
                nano: 450_000_000
            }
        );
    }

    #[test]
    fn test_quotation_to_money_value() {
        let quotation = Quotation {
            units: -7,
            nano: -80_000_000,
        };

        assert_eq!(
            quotation_and_currency_to_money_value(&quotation, "USD"),
            MoneyValue {
                currency: "USD".to_string(),
                units: -7,
                nano: -80_000_000
            }
        );
    }

    #[test]
    fn test_money_value_to_decimal() {
        let money = MoneyValue {
            currency: "RUB".to_string(),
            units: 114,
            nano: 250_000_000,
        };

        assert_eq!(money_value_to_decimal(&money), dec!(114.25));
    }

    #[test]
    fn test_decimal_to_money_value() {
        assert_eq!(
            decimal_and_currency_to_money_value(dec!(-200.20), "EUR"),
            MoneyValue {
                currency: "EUR".to_string(),
                units: -200,
                nano: -200_000_000
            }
        );
    }
}
