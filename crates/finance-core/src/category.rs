use crate::{error::DomainError, ids::CategoryId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: CategoryId,
    pub name: String,
    pub icon: Option<String>,
    pub color: String,
    pub is_active: bool,
    pub sort_order: i32,
}

pub fn validate_name(name: &str) -> Result<(), DomainError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 40 {
        Err(DomainError::InvalidCategoryName)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_name_must_not_be_empty() {
        assert!(validate_name("Comida").is_ok());
        assert!(validate_name("   ").is_err());
    }
}
