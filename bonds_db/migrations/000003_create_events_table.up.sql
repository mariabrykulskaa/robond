CREATE TABLE events (
    instrument_uid uuid PRIMARY KEY,
    events bytea NOT NULL
);
