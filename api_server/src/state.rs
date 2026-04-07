use portfolio::PortfolioClient;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub portfolio_client: PortfolioClient,
    pub jwt_secret: String,
}
