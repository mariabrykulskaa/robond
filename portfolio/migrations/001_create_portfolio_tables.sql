-- Портфель пользователя
CREATE TABLE IF NOT EXISTS portfolio (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

-- Позиции: сколько облигаций каждого типа в портфеле
CREATE TABLE IF NOT EXISTS portfolio_holding (
    id              BIGSERIAL PRIMARY KEY,
    portfolio_id    BIGINT NOT NULL REFERENCES portfolio(id) ON DELETE CASCADE,
    isin            TEXT NOT NULL,
    quantity        BIGINT NOT NULL DEFAULT 0,
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    UNIQUE (portfolio_id, isin)
);

-- Свободные денежные средства
CREATE TABLE IF NOT EXISTS portfolio_cash (
    id              BIGSERIAL PRIMARY KEY,
    portfolio_id    BIGINT NOT NULL REFERENCES portfolio(id) ON DELETE CASCADE UNIQUE,
    amount          NUMERIC(20, 6) NOT NULL DEFAULT 0,
    currency        TEXT NOT NULL DEFAULT 'RUB',
    updated_at      TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

-- Снимки рыночной стоимости портфеля во времени (для графика и расчёта доходности)
CREATE TABLE IF NOT EXISTS portfolio_snapshot (
    id              BIGSERIAL PRIMARY KEY,
    portfolio_id    BIGINT NOT NULL REFERENCES portfolio(id) ON DELETE CASCADE,
    date            DATE NOT NULL,
    market_value    NUMERIC(20, 6) NOT NULL,
    cash            NUMERIC(20, 6) NOT NULL DEFAULT 0,
    bonds_value     NUMERIC(20, 6) NOT NULL DEFAULT 0,
    UNIQUE (portfolio_id, date)
);

CREATE INDEX IF NOT EXISTS idx_portfolio_snapshot_date ON portfolio_snapshot (portfolio_id, date);
CREATE INDEX IF NOT EXISTS idx_portfolio_holding_portfolio ON portfolio_holding (portfolio_id);
