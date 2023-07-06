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
    LogicOp(LogicOperator, Box<ASTNode>, Box<ASTNode>), // Branch node for logical operators
    ArithmeticOp(ArithmeticOperator, Box<ASTNode>, Box<ASTNode>), // Branch node for arithmetic operators
}

/// AST Value which contains a constant value
#[derive(Debug, Clone)]
pub enum ASTValue {
    Bool(bool),
    Number(f64),
    String(String),
}

impl ASTNode {
    pub fn evaluate(&self) -> Result<ASTValue, &'static str> {
        match self {
            ASTNode::ConstantBool(val) => Ok(ASTValue::Bool(*val)),
            ASTNode::ConstantNumber(val) => Ok(ASTValue::Number(*val)),
            ASTNode::ConstantString(val) => Ok(ASTValue::String(val.clone())),
            ASTNode::LogicOp(op, left, right) => {
                let left_val = left.evaluate()?;
                let right_val = right.evaluate()?;
                match op {
                    LogicOperator::Greater => {
                        if let (ASTValue::Number(left), ASTValue::Number(right)) =
                            (left_val, right_val)
                        {
                            Ok(ASTValue::Bool(left > right))
                        } else {
                            Err("Greater operator requires number operands.")
                        }
                    }
                    LogicOperator::Less => {
                        if let (ASTValue::Number(left), ASTValue::Number(right)) =
                            (left_val, right_val)
                        {
                            Ok(ASTValue::Bool(left < right))
                        } else {
                            Err("Less operator requires number operands.")
                        }
                    }
                    LogicOperator::GreaterOrEqual => {
                        if let (ASTValue::Number(left), ASTValue::Number(right)) =
                            (left_val, right_val)
                        {
                            Ok(ASTValue::Bool(left >= right))
                        } else {
                            Err("Greater or equal operator requires number operands.")
                        }
                    }
                    LogicOperator::LessOrEqual => {
                        if let (ASTValue::Number(left), ASTValue::Number(right)) =
                            (left_val, right_val)
                        {
                            Ok(ASTValue::Bool(left <= right))
                        } else {
                            Err("Less or equal operator requires number operands.")
                        }
                    }
                    LogicOperator::Equal => {
                        if let (ASTValue::Number(left), ASTValue::Number(right)) =
                            (left_val, right_val)
                        {
                            Ok(ASTValue::Bool(left == right))
                        } else {
                            Err("Equal operator requires number operands.")
                        }
                    }
                    LogicOperator::NotEqual => {
                        if let (ASTValue::Number(left), ASTValue::Number(right)) =
                            (left_val, right_val)
                        {
                            Ok(ASTValue::Bool(left != right))
                        } else {
                            Err("Not equal operator requires number operands.")
                        }
                    }
                    _ => Err("Unsupported logic operator."),
                }
            },
            ASTNode::ArithmeticOp(op, left, right) => {
                let left_val = left.evaluate()?;
                let right_val = right.evaluate()?;
                match op {
                    ArithmeticOperator::Add =>if let (ASTValue::Number(left), ASTValue::Number(right)) = (left_val, right_val) {
                        Ok(ASTValue::Number(left + right))
                    }else{
                        Err("Arithmetic ADD operator requires number operands.")
                    },
                    ArithmeticOperator::Subtract =>if let (ASTValue::Number(left), ASTValue::Number(right)) = (left_val, right_val) {
                        Ok(ASTValue::Number(left - right))
                    }else{
                        Err("Arithmetic SUBTRACT operator requires number operands.")
                    },
                    ArithmeticOperator::Multiply =>if let (ASTValue::Number(left), ASTValue::Number(right)) = (left_val, right_val) {
                        Ok(ASTValue::Number(left * right))
                    }else{
                        Err("Arithmetic MULTIPLY operator requires number operands.")
                    },
                    ArithmeticOperator::Divide =>if let (ASTValue::Number(left), ASTValue::Number(right)) = (left_val, right_val) {
                        Ok(ASTValue::Number(left / right))
                    }else{
                        Err("Arithmetic DIVIDE operator requires number operands.")
                    },
                    ArithmeticOperator::Modulo =>if let (ASTValue::Number(left), ASTValue::Number(right)) = (left_val, right_val) {
                        Ok(ASTValue::Number(left % right))
                    }else{
                        Err("Arithmetic MODULO operator requires number operands.")
                    }
                }
            }
        }
    }
}

#[test]
fn test_ast(){
    // Example: 5 + 5 > 17 - 15

    let exp_a = ASTNode::Constant::<u64>(5);
    let exp_b = ASTNode::Constant::<u64>(5);
    let exp_c = ASTNode::Constant::<u64>(17);
    let exp_d = ASTNode::Constant::<u64>(15);

    let exp_e = ASTNode::ArithmeticOp(ArithmeticOperator::Add, Box::new(exp_a), Box::new(exp_b));
    let exp_f = ASTNode::ArithmeticOp(ArithmeticOperator::Subtract, Box::new(exp_c), Box::new(exp_d));
    let exp_g = ASTNode::LogicOp(LogicOperator::Greater, Box::new(exp_e), Box::new(exp_f));

    assert_eq!(exp_g.evaluate_bool().unwrap(), true);
}