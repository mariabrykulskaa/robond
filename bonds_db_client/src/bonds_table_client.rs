use t_invest_api_rust::proto::Bond;

use prost::Message;
use sqlx::PgPool;
use uuid::Uuid;

use crate::Result;

#[derive(Debug)]
struct MyBond {
    bond: Vec<u8>,
}

pub struct BondsTableClient {
    pool: PgPool,
}

impl BondsTableClient {
    pub fn new(pool: PgPool) -> Self {
        BondsTableClient { pool }
    }

    pub async fn clear(&self) -> Result<()> {
        sqlx::query!("TRUNCATE TABLE bonds").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn read(&self) -> Result<Vec<Bond>> {
        let bonds = sqlx::query_as!(MyBond, "SELECT bond FROM bonds ORDER BY instrument_uid")
            .fetch_all(&self.pool)
            .await?;

        Ok(bonds
            .into_iter()
            .map(|bond| Bond::decode(bond.bond.as_slice()))
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn insert(&self, bonds: &[Bond]) -> Result<()> {
        let instrument_uids = bonds
            .iter()
            .map(|bond| Uuid::parse_str(&bond.uid))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let bonds = bonds
            .iter()
            .map(|bond| {
                let mut bond_bytes = Vec::<u8>::new();
                bond.encode(&mut bond_bytes).unwrap();
                bond_bytes
            })
            .collect::<Vec<Vec<u8>>>();

        sqlx::query!(
            r#"
            INSERT INTO bonds (instrument_uid, bond)
            SELECT * FROM UNNEST($1::uuid[], $2::bytea[])
        "#,
            &instrument_uids,
            &bonds,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
