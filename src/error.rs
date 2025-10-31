use std::fmt;

#[derive(Debug)]
pub enum EmulatorErrorType {
    AudioInitFailed,
}

#[derive(Debug)]
pub struct EmulatorError {
    pub error_type: EmulatorErrorType,
    pub info: String
}
impl fmt::Display for EmulatorErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EmulatorErrorType::AudioInitFailed => {
                write!(f, "Audio initialization failed!")
            }
        }
    }
}


impl EmulatorError {
    pub fn new(error_type: EmulatorErrorType, info: String) -> EmulatorError {
        Self {
            error_type,
            info
        }
    }
}

impl fmt::Display for EmulatorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out = format!("Error Type: {}\nInfo: {}", self.error_type, self.info);
        write!(f, "{}", out)
    }
}
impl std::error::Error for EmulatorError {}