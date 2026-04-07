-- T-Invest токен и аккаунт пользователя
ALTER TABLE app_user ADD COLUMN IF NOT EXISTS tinvest_token TEXT;
ALTER TABLE app_user ADD COLUMN IF NOT EXISTS tinvest_account_id TEXT;
ALTER TABLE app_user ADD COLUMN IF NOT EXISTS tinvest_endpoint TEXT DEFAULT 'sandbox';

-- Привязка стратегии к портфелю
ALTER TABLE portfolio ADD COLUMN IF NOT EXISTS strategy_name TEXT;
ALTER TABLE portfolio ADD COLUMN IF NOT EXISTS strategy_running BOOLEAN DEFAULT false;
