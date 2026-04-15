use t_invest_api_rust::proto::{Coupon, GetBondCouponsResponse};

use prost::Message;
use sqlx::PgPool;
use uuid::Uuid;

use crate::Result;

#[derive(Debug)]
struct MyCoupons {
    instrument_uid: Uuid,
    coupons: Vec<u8>,
}

pub struct CouponsTableClient {
    pool: PgPool,
}

impl CouponsTableClient {
    pub fn new(pool: PgPool) -> Self {
        CouponsTableClient { pool }
    }

    pub async fn clear(&self) -> Result<()> {
        sqlx::query!("TRUNCATE TABLE coupons").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn read(&self) -> Result<Vec<(Uuid, Vec<Coupon>)>> {
        let coupons = sqlx::query_as!(
            MyCoupons,
            "SELECT instrument_uid, coupons FROM coupons ORDER BY instrument_uid"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(coupons
            .into_iter()
            .map(|coupons| {
                GetBondCouponsResponse::decode(coupons.coupons.as_slice()).map(|c| (coupons.instrument_uid, c.events))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn insert(&self, instrument_uids: &[Uuid], coupons: &[Vec<Coupon>]) -> Result<()> {
        assert_eq!(instrument_uids.len(), coupons.len());
        let coupons = coupons
            .iter()
            .map(|coupons| {
                let coupons = GetBondCouponsResponse {
                    events: coupons.clone(),
                };
                let mut coupons_bytes = Vec::<u8>::new();
                coupons.encode(&mut coupons_bytes).unwrap();
                coupons_bytes
            })
            .collect::<Vec<Vec<u8>>>();

        sqlx::query!(
            r#"
            INSERT INTO coupons (instrument_uid, coupons)
            SELECT * FROM UNNEST($1::uuid[], $2::bytea[])
        "#,
            &instrument_uids,
            &coupons,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
