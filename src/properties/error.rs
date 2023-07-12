use thiserror::Error;

#[derive(Error, Debug)]
pub enum ASTError {
    #[error("the supplied operator {0} is no valid logical operator")]
    InvalidLogicOperator(String),
    #[error("the supplied operator {0} is no valid arithmetic operator")]
    InvalidArithmeticOperator(String),
    #[error("the variable {var:?} does not exist")]
    VariableNotFound { var: String },
    #[error("can't use operator {0} on type {1} and type {2}")]
    InvalidOperation(String, String, String),
    #[error("operator is not supported as a unary operator")]
    InvalidUnaryOperator,
    #[error("operator is not supported as a binary operator")]
    InvalidBinaryOperator,
    #[error("invalid constant for the operator {0}")]
    InvalidConstant(String),
}