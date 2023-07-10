/// This file describes an Abstract Syntax Tree which should contain as leaves constants and the branches refer to logical or arithmetic operators.


/// A Logical Operator which refers to Logical Statements returning either true or false
#[derive(Debug, Clone, PartialEq)]
pub enum LogicOperator {
    Greater,
    Less,
    GreaterOrEqual,
    LessOrEqual,
    Equal,
    NotEqual,
    And,
    Or,
    Not,
}

impl LogicOperator {
    pub fn to_string(&self) -> &str {
        match self {
            LogicOperator::Greater => ">",
            LogicOperator::Less => "<",
            LogicOperator::GreaterOrEqual => ">=",
            LogicOperator::LessOrEqual => "<=",
            LogicOperator::Equal => "==",
            LogicOperator::NotEqual => "!=",
            LogicOperator::And => "&&",
            LogicOperator::Or => "||",
            LogicOperator::Not => "!",
        }
    }

    pub fn from_str(string: &str) -> Result<LogicOperator, &'static str> {
        match string {
            ">" => Ok(LogicOperator::Greater),
            "<" => Ok(LogicOperator::Less),
            ">=" => Ok(LogicOperator::GreaterOrEqual),
            "<=" => Ok(LogicOperator::LessOrEqual),
            "==" => Ok(LogicOperator::Equal),
            "!=" => Ok(LogicOperator::NotEqual),
            "&&" => Ok(LogicOperator::And),
            "||" => Ok(LogicOperator::Or),
            "!" => Ok(LogicOperator::Not),
            _ => Err("Invalid Logic Operator"),
        }
    }
}

/// An Arithmetic Operator which refers to Statements returning a constant value
#[derive(Debug, Clone, PartialEq)]
pub enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

impl ArithmeticOperator{
    pub fn to_string(&self) -> &str {
        match self {
            ArithmeticOperator::Add => "+",
            ArithmeticOperator::Subtract => "-",
            ArithmeticOperator::Multiply => "*",
            ArithmeticOperator::Divide => "/",
            ArithmeticOperator::Modulo => "%",
        }
    }

    pub fn from_str(string: &str) -> Result<ArithmeticOperator, &'static str> {
        match string {
            "+" => Ok(ArithmeticOperator::Add),
            "-" => Ok(ArithmeticOperator::Subtract),
            "*" => Ok(ArithmeticOperator::Multiply),
            "/" => Ok(ArithmeticOperator::Divide),
            "%" => Ok(ArithmeticOperator::Modulo),
            _ => Err("Invalid Arithmetic Operator"),
        }
    }
}

/// AST Node which contains leaves and branches for an Abstract Syntax Tree
#[derive(Debug, Clone)]
pub enum ASTNode {
    ConstantBool(bool),
    ConstantNumber(f64),
    ConstantString(String),
    UnaryArithmetic(ArithmeticOperator, Box<ASTNode>),
    BinaryArithmetic(ArithmeticOperator, Box<ASTNode>, Box<ASTNode>),
    UnaryLogic(LogicOperator, Box<ASTNode>),
    BinaryLogic(LogicOperator, Box<ASTNode>, Box<ASTNode>),
}

/// AST Value which contains a constant value
#[derive(Debug, Clone, PartialEq)]
pub enum ASTConstant {
    Bool(bool),
    Number(f64),
    String(String),
}

impl ASTConstant{
    pub fn get_constant_info(&self) -> (&str, String) {
        match self {
            ASTConstant::Bool(value) => ("Bool", value.to_string()),
            ASTConstant::Number(value) => ("Number", value.to_string()),
            ASTConstant::String(value) => ("String", value.clone()),
        }
    }
}

impl ASTNode {
    pub fn evaluate(&self) -> Result<ASTConstant, &'static str> {
        match self {
            ASTNode::ConstantBool(value) => Ok(ASTConstant::Bool(*value)),
            ASTNode::ConstantNumber(value) => Ok(ASTConstant::Number(*value)),
            ASTNode::ConstantString(value) => Ok(ASTConstant::String(value.clone())),
            ASTNode::UnaryArithmetic(operator, value) => { // Implementation of Unary Arithmetic Operations
                let val = value.evaluate()?;
                match operator { 
                    ArithmeticOperator::Subtract => {
                        match val {
                            ASTConstant::Number(value) => Ok(ASTConstant::Number(-value)),
                            _ => Err("Not Implemented"),
                        }
                    },
                    _ => Err("Not Implemented"),
                }
            }
            ASTNode::BinaryArithmetic(operator, left, right) => { // Implementation of Binary Arithmetic Operations
                let left = left.evaluate()?;
                let right = right.evaluate()?;
                match left {
                    ASTConstant::Number(left) => {
                        match right {
                            ASTConstant::Number(right) => {
                                match operator {
                                    ArithmeticOperator::Add => Ok(ASTConstant::Number(left + right)),
                                    ArithmeticOperator::Subtract => Ok(ASTConstant::Number(left - right)),
                                    ArithmeticOperator::Multiply => Ok(ASTConstant::Number(left * right)),
                                    ArithmeticOperator::Divide => Ok(ASTConstant::Number(left / right)),
                                    ArithmeticOperator::Modulo => Ok(ASTConstant::Number(left % right)),
                                    _ => Err("Not Implemented"),
                                }
                            },
                            _ => Err("Not Implemented"),
                        }
                    }
                    _ => Err("Not Implemented"),
                }
            },
            ASTNode::UnaryLogic(operator, value) => {
                let val = value.evaluate()?;
                match val {
                    ASTConstant::Bool(value) => {
                        match operator {
                        LogicOperator::Not => Ok(ASTConstant::Bool(!value)),
                        _ => Err("Not Implemented"),
                        }
                    },
                    _ => Err("Not Implemented"),
                }
            },
            ASTNode::BinaryLogic(operator, left, right) => {
                let left = left.evaluate()?;
                let right = right.evaluate()?;
                match left {
                    ASTConstant::Bool(left) => {
                        match right {
                            ASTConstant::Bool(right) => {
                                match operator {
                                    LogicOperator::And => Ok(ASTConstant::Bool(left && right)),
                                    LogicOperator::Or => Ok(ASTConstant::Bool(left || right)),
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    _ => Err("Not Implemented"),
                                }
                            },
                            _ => Err("Not Implemented"),
                        }
                    },
                    ASTConstant::Number(left) => {
                        match right {
                            ASTConstant::Number(right) => {
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    LogicOperator::Greater => Ok(ASTConstant::Bool(left > right)),
                                    LogicOperator::Less => Ok(ASTConstant::Bool(left < right)),
                                    LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left >= right)),
                                    LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left <= right)),
                                    _ => Err("Not Implemented"),
                                }
                            },
                            _ => Err("Not Implemented"),
                        }
                    },
                    ASTConstant::String(left) => {
                        match right {
                            ASTConstant::String(right) => {
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    _ => Err("Not Implemented"),
                                }
                            },
                            _ => Err("Not Implemented"),
                        }
                    }
                }
            }
            _ => Err("Not Implemented"),
        }
    }
}


#[test]
fn test_ast(){
    // Example: 5 + 5 > 17 - 15

    let exp_a = ASTNode::ConstantNumber(5.0);
    let exp_b = ASTNode::ConstantNumber(5.0);
    let exp_a_b = ASTNode::BinaryArithmetic(ArithmeticOperator::Add, Box::new(exp_a), Box::new(exp_b));

    let exp_c = ASTNode::ConstantNumber(17.0);
    let exp_d = ASTNode::ConstantNumber(15.0);
    let exp_c_d = ASTNode::BinaryArithmetic(ArithmeticOperator::Subtract, Box::new(exp_c), Box::new(exp_d));

    let exp_g = ASTNode::BinaryLogic(LogicOperator::Greater, Box::new(exp_a_b), Box::new(exp_c_d));
    
    let val = exp_g.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);

    assert_eq!(value, "true");
}