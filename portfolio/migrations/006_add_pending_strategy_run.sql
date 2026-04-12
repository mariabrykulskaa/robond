-- Флаг отложенного запуска стратегии (выполнится при открытии биржи)
ALTER TABLE portfolio ADD COLUMN IF NOT EXISTS pending_strategy_run BOOLEAN NOT NULL DEFAULT FALSE;
