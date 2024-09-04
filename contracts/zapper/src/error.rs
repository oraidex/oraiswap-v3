use std::string::FromUtf8Error;

use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    FromUtf8(#[from] FromUtf8Error),

    #[error("{0}")]
    CheckMathOverUnderFlowError(String),

    #[error("Unauthorized")]
    Unauthorized {},
}

impl From<ContractError> for StdError {
    fn from(source: ContractError) -> Self {
        Self::generic_err(source.to_string())
    }
}

// Implementing From<String> for ContractError
impl From<String> for ContractError {
    fn from(error: String) -> Self {
        ContractError::CheckMathOverUnderFlowError(error)
    }
}

// Implementing From<&str> for ContractError
impl From<&str> for ContractError {
    fn from(error: &str) -> Self {
        ContractError::CheckMathOverUnderFlowError(error.to_string())
    }
}