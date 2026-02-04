pub struct ErrorInfo {
    pub(crate) context: String,
    pub(crate) details: String,
}

impl ErrorInfo {
    pub fn new(context: impl Into<String>, details: impl Into<String>) -> Self {
        Self {
            context: context.into(),
            details: details.into(),
        }
    }
    pub fn from_anyhow(context: impl Into<String>, err: anyhow::Error) -> Self {
        let context = context.into();
        let details = format!("{err:#}");
        Self { context, details }
    }
}
