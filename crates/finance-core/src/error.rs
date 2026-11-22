use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("la divisa debe tener 3 letras mayúsculas (ISO 4217)")]
    InvalidCurrency,
    #[error("el importe debe ser mayor que cero")]
    InvalidAmount,
    #[error("la nota no puede superar los 200 caracteres")]
    NoteTooLong,
    #[error("el nombre de la categoría debe tener entre 1 y 40 caracteres")]
    InvalidCategoryName,
    #[error("el nombre del pago debe tener entre 1 y 40 caracteres")]
    InvalidPaymentName,
    #[error("el día de vencimiento debe estar entre 1 y 31")]
    InvalidDueDay,
    #[error("la fecha local no coincide con la fecha del movimiento")]
    DateMismatch,
    #[error("no se pueden mezclar divisas en un reporte")]
    MixedCurrencies,
    #[error("el total se desbordó")]
    Overflow,
    #[error("categoría desconocida: {0}")]
    UnknownCategory(i64),
}
