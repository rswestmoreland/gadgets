use crate::error::ValidationError;

pub trait Validate {
    fn validate(&self) -> ValidationReport;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
}

impl ValidationReport {
    pub fn push(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }
}
