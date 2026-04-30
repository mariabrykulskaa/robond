-- Перенос T-Invest токена с пользователя на портфель
ALTER TABLE portfolio ADD COLUMN IF NOT EXISTS tinvest_token TEXT;
ALTER TABLE portfolio ADD COLUMN IF NOT EXISTS tinvest_account_id TEXT;
ALTER TABLE portfolio ADD COLUMN IF NOT EXISTS tinvest_endpoint TEXT DEFAULT 'sandbox';

-- Мигрируем существующие данные: копируем токен пользователя во все его портфели
UPDATE portfolio p
SET tinvest_token = u.tinvest_token,
    tinvest_account_id = u.tinvest_account_id,
    tinvest_endpoint = u.tinvest_endpoint
FROM app_user u
WHERE p.user_id = u.id AND u.tinvest_token IS NOT NULL;

-- Удаляем колонки с таблицы пользователя
ALTER TABLE app_user DROP COLUMN IF EXISTS tinvest_token;
ALTER TABLE app_user DROP COLUMN IF EXISTS tinvest_account_id;
ALTER TABLE app_user DROP COLUMN IF EXISTS tinvest_endpoint;
