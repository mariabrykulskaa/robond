-- Привязка портфелей к пользователям
ALTER TABLE portfolio ADD COLUMN IF NOT EXISTS user_id BIGINT REFERENCES app_user(id);

CREATE INDEX IF NOT EXISTS idx_portfolio_user_id ON portfolio (user_id);
