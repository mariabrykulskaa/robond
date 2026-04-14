use sqlx::PgPool;

use crate::Result;

pub struct UserManager {
    pool: PgPool,
}

impl UserManager {
    pub fn new(pool: PgPool) -> Self {
        UserManager { pool }
    }

    pub async fn setup_roles(&self) -> Result<()> {
        // Проверка readonly
        let readonly_exists = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM pg_roles WHERE rolname = 'readonly'
            ) as "exists!"
            "#
        )
        .fetch_one(&self.pool)
        .await?
        .exists;

        if !readonly_exists {
            sqlx::query("CREATE ROLE readonly").execute(&self.pool).await?;
        }

        // Проверка readwrite
        let readwrite_exists = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM pg_roles WHERE rolname = 'readwrite'
            ) as "exists!"
            "#
        )
        .fetch_one(&self.pool)
        .await?
        .exists;

        if !readwrite_exists {
            sqlx::query("CREATE ROLE readwrite").execute(&self.pool).await?;
        }

        // Права readonly
        sqlx::query("GRANT CONNECT ON DATABASE bonds_db TO readonly")
            .execute(&self.pool)
            .await?;
        sqlx::query("GRANT USAGE ON SCHEMA public TO readonly")
            .execute(&self.pool)
            .await?;
        sqlx::query("GRANT SELECT ON ALL TABLES IN SCHEMA public TO readonly")
            .execute(&self.pool)
            .await?;

        // Права readwrite
        sqlx::query("GRANT CONNECT ON DATABASE bonds_db TO readwrite")
            .execute(&self.pool)
            .await?;
        sqlx::query("GRANT USAGE ON SCHEMA public TO readwrite")
            .execute(&self.pool)
            .await?;
        sqlx::query("GRANT SELECT, INSERT, UPDATE, DELETE, TRUNCATE ON ALL TABLES IN SCHEMA public TO readwrite")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        role: &str, // "readonly" | "readwrite"
    ) -> Result<()> {
        // 1. Проверяем, существует ли пользователь
        let exists = sqlx::query!(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM pg_roles WHERE rolname = $1
            ) as "exists!"
            "#,
            username
        )
        .fetch_one(&self.pool)
        .await?
        .exists;

        assert!(!exists);

        let create_user_sql = format!("CREATE USER {} WITH PASSWORD '{}'", username, password);

        sqlx::query(&create_user_sql).execute(&self.pool).await?;

        // 3. Назначаем роль
        let grant_sql = format!("GRANT {} TO {}", role, username);

        sqlx::query(&grant_sql).execute(&self.pool).await?;

        Ok(())
    }
}
