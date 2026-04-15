use t_invest_api_rust::proto::{GetBondEventsResponse, get_bond_events_response::BondEvent};

use prost::Message;
use sqlx::PgPool;
use uuid::Uuid;

use crate::Result;

#[derive(Debug)]
struct Events {
    instrument_uid: Uuid,
    events: Vec<u8>,
}

pub struct EventsTableClient {
    pool: PgPool,
}

impl EventsTableClient {
    pub fn new(pool: PgPool) -> Self {
        EventsTableClient { pool }
    }

    pub async fn clear(&self) -> Result<()> {
        sqlx::query!("TRUNCATE TABLE events").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn read(&self) -> Result<Vec<(Uuid, Vec<BondEvent>)>> {
        let events = sqlx::query_as!(
            Events,
            "SELECT instrument_uid, events FROM events ORDER BY instrument_uid"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(events
            .into_iter()
            .map(|events| {
                GetBondEventsResponse::decode(events.events.as_slice()).map(|e| (events.instrument_uid, e.events))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn insert(&self, instrument_uids: &[Uuid], events: &[Vec<BondEvent>]) -> Result<()> {
        assert_eq!(instrument_uids.len(), events.len());
        let events = events
            .iter()
            .map(|events| {
                let events = GetBondEventsResponse { events: events.clone() };
                let mut events_bytes = Vec::<u8>::new();
                events.encode(&mut events_bytes).unwrap();
                events_bytes
            })
            .collect::<Vec<Vec<u8>>>();

        sqlx::query!(
            r#"
            INSERT INTO events (instrument_uid, events)
            SELECT * FROM UNNEST($1::uuid[], $2::bytea[])
        "#,
            &instrument_uids,
            &events,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
