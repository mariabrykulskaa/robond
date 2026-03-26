//! Преобразования между protobuf-типом `Quotation` и `rust_decimal::Decimal`.
//!
//! Модуль доступен только при включённой feature `decimal`.

use rust_decimal::prelude::*;

use crate::proto::Quotation;

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
}
