use crate::recurring::PaymentMethod;

/// Canal usado para un ingreso o un gasto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MovementMethod {
    Cash,
    BankTransfer,
    BankMobile,
    ExternalPage,
    #[default]
    Other,
}

impl MovementMethod {
    pub const ALL: [Self; 5] = [
        Self::Cash,
        Self::BankTransfer,
        Self::BankMobile,
        Self::ExternalPage,
        Self::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::BankTransfer => "bank_transfer",
            Self::BankMobile => "bank_mobile",
            Self::ExternalPage => "external",
            Self::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Cash => "Efectivo",
            Self::BankTransfer => "Banco · Transferencia",
            Self::BankMobile => "Banco · Pago móvil",
            Self::ExternalPage => "Página externa",
            Self::Other => "Otro",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::BankTransfer => "transfer",
            Self::BankMobile => "card",
            Self::ExternalPage => "shopping",
            Self::Other => "other",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.trim() {
            "cash" | "efectivo" => Self::Cash,
            "bank_transfer" | "transfer" | "transferencia" => Self::BankTransfer,
            "bank_mobile" | "pago_movil" | "pago móvil" => Self::BankMobile,
            "external" | "pagina" | "página externa" => Self::ExternalPage,
            _ => Self::Other,
        }
    }

    /// Los pagos recurrentes tienen su propio método (por ejemplo Cashea); al
    /// convertirlos en gasto se conserva el canal con el que se pagaron.
    pub fn from_payment_method(method: PaymentMethod) -> Self {
        match method {
            PaymentMethod::Cash => Self::Cash,
            PaymentMethod::Transfer => Self::BankTransfer,
            PaymentMethod::Cashea => Self::ExternalPage,
            PaymentMethod::Other => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_labels_every_method() {
        for method in MovementMethod::ALL {
            assert_eq!(MovementMethod::parse(method.as_str()), method);
            assert!(!method.label().is_empty());
        }
        assert_eq!(
            MovementMethod::parse("transferencia"),
            MovementMethod::BankTransfer
        );
        assert_eq!(MovementMethod::parse(""), MovementMethod::Other);
    }

    #[test]
    fn maps_payment_methods_without_losing_cashea() {
        assert_eq!(
            MovementMethod::from_payment_method(PaymentMethod::Cashea),
            MovementMethod::ExternalPage
        );
        assert_eq!(PaymentMethod::parse("cashea"), PaymentMethod::Cashea);
    }
}
