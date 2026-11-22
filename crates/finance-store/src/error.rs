use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("error de base de datos: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("error en migración: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    #[error("regla de negocio: {0}")]
    Domain(#[from] finance_core::DomainError),
}
