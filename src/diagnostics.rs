use std::fmt::{self, Display};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub location: Option<String>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: "error".to_string(),
            location: None,
        }
    }

    pub fn at(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }
}

impl Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.location {
            Some(location) => write!(
                f,
                "{} {}: {}: {}",
                self.severity.to_uppercase(),
                self.code,
                location,
                self.message
            ),
            None => write!(
                f,
                "{} {}: {}",
                self.severity.to_uppercase(),
                self.code,
                self.message
            ),
        }
    }
}
