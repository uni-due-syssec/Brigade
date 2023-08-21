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
    #[error("invalid function called {0}")]
    InvalidFunction(String),
    #[error("can't parse array without delimiter")]
    InvalidArray,
    #[error("can't invoke function {0}")]
    InvalidFunctionInvocation(String),
    #[error("can't invoke function {0} because of missing parameters")]
    InvalidFunctionParameter(String),
    #[error("can't convert {0} to {1}")]
    InvalidConversion(String, String),
    #[error("can't use function as with parameter {0}. Use only strings containing the conversion target")]
    InvalidConversionTarget(String),
    #[error("unknown conversion target {0}")]
    UnknownConversionTarget(String),
    #[error("can't slice out of bounds for {0} from {1} to {2} for length {3}")]
    InvalidSlice(String, usize, usize, usize),
}

#[derive(Error, Debug)]
pub enum PropertyError {
    #[error("the property is invalid")]
    InvalidProperty,  
    #[error("the fieldname does not exist or an invalid response was returned")]
    FieldNotFound, 
    #[error("the property is not found")]
    PropertyNotFound,
    #[error("cyclic dependencies detected")]
    CyclicDependencies,
}