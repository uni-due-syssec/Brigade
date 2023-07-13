use core::panic;
use std::collections::VecDeque;
use std::fmt::format;

use crate::{get_var, set_var};

use super::error::{self, ASTError};

use super::environment::{get_variable, VariableMap, get_variable_map_instance, VarValues};
use std::str::FromStr;
use regex::Regex;

/// This file describes an Abstract Syntax Tree which should contain as leaves constants and the branches refer to logical or arithmetic operators.
/// The AST consists of Nodes see ASTNode struct
/// When evaluating the AST an ASTConstant is returned. See ASTConstant struct


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

    pub fn from_str(string: &str) -> Result<LogicOperator, ASTError> {
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
            _ => Err(ASTError::InvalidLogicOperator(string.to_owned())),
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
    Negate,
}

impl ArithmeticOperator{
    pub fn to_string(&self) -> &str {
        match self {
            ArithmeticOperator::Add => "+",
            ArithmeticOperator::Subtract => "-",
            ArithmeticOperator::Multiply => "*",
            ArithmeticOperator::Divide => "/",
            ArithmeticOperator::Modulo => "%",
            ArithmeticOperator::Negate => "neg",
        }
    }

    pub fn from_str(string: &str) -> Result<ArithmeticOperator, ASTError> {
        match string {
            "+" => Ok(ArithmeticOperator::Add),
            "-" => Ok(ArithmeticOperator::Subtract),
            "*" => Ok(ArithmeticOperator::Multiply),
            "/" => Ok(ArithmeticOperator::Divide),
            "%" => Ok(ArithmeticOperator::Modulo),
            "neg" => Ok(ArithmeticOperator::Negate),
            _ => Err(ASTError::InvalidArithmeticOperator(string.to_owned())),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Functions{
    Contains, // Returns true if ASTNode Array contains a ASTNode value
    At, // Returns the ASTNode value at index
}

impl Functions{
    pub fn to_string(&self) -> &str {
        match self {
            Functions::Contains => "contains",
            Functions::At => "at",
        }
    }

    pub fn from_str(string: &str) -> Result<Functions, ASTError> {
        match string {
            "contains" => Ok(Functions::Contains),
            "at" => Ok(Functions::At),
            _ => Err(ASTError::InvalidFunction(string.to_owned())),
        }
    }

    pub fn get_args(string: &str) -> Option<Vec<String>> {
        let s = string[0..string.len()-1].to_owned();
        Some(s.split(",").map(|s| s.trim().to_string()).collect())
    }
}

/// AST Node which contains leaves and branches for an Abstract Syntax Tree
#[derive(Debug, Clone)]
pub enum ASTNode {

    // Constants
    ConstantBool(bool),
    ConstantNumber(f64),
    ConstantString(String),
    Array(Vec<Box<ASTNode>>),
    // arr.foreach() 
    // arr.contains()
    // arr[0] as ASTNode 
    
    // Variable
    Variable(String, &'static VariableMap), // String points to a variable on the VariableMap
    
    // Operators
    UnaryArithmetic(ArithmeticOperator, Box<ASTNode>),
    BinaryArithmetic(ArithmeticOperator, Box<ASTNode>, Box<ASTNode>),
    UnaryLogic(LogicOperator, Box<ASTNode>),
    BinaryLogic(LogicOperator, Box<ASTNode>, Box<ASTNode>),

    // Functions
    Function(Functions, Vec<Box<ASTNode>>),
}

/// AST Value which contains a constant value
#[derive(Debug, Clone, PartialEq)]
pub enum ASTConstant {
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<ASTConstant>),
}

impl ASTConstant{
    pub fn get_constant_info(&self) -> (&str, String) {
        match self {
            ASTConstant::Bool(value) => ("Bool", value.to_string()),
            ASTConstant::Number(value) => ("Number", value.to_string()),
            ASTConstant::String(value) => ("String", value.clone()),
            ASTConstant::Array(value) => ("Array", format!("{:?}", value)),
        }
    }

    pub fn get_value(&self) -> String {
        match self {
            ASTConstant::Bool(value) => value.to_string(),
            ASTConstant::Number(value) => value.to_string(),
            ASTConstant::String(value) => value.clone(),
            ASTConstant::Array(value) => {
                let s = value.iter().map(|value| value.get_value()).collect::<Vec<String>>().join(",");
                return format!("[{}]", s);
            }
        }
    }

    pub fn parse(value: String) -> Self {
        if value.starts_with("["){
            let v = value[1..value.len()-1].split(",").map(|x: &str| x.to_string()).collect::<Vec<String>>();
            let mut arr = Vec::new();
            for s in v.iter(){
                arr.push(ASTConstant::parse(s.to_string()));
            }
            ASTConstant::Array(arr)
        }else{
            match value.parse::<f64>() {
                Ok(value) => ASTConstant::Number(value),
                Err(_) => {
                    match value.parse::<bool>() {
                        Ok(value) => ASTConstant::Bool(value),
                        Err(_) => ASTConstant::String(value),
                    }
                },
            }
        }
    }
}

impl ASTNode {
    pub fn evaluate(&self) -> Result<ASTConstant, ASTError> {
        match self {
            ASTNode::ConstantBool(value) => Ok(ASTConstant::Bool(*value)),
            ASTNode::ConstantNumber(value) => Ok(ASTConstant::Number(*value)),
            ASTNode::ConstantString(value) => Ok(ASTConstant::String(value.clone())),
            ASTNode::Variable(name, map_ref) => {

                match get_variable(map_ref, name) {
                    Some(value) => {
                        println!("{:?}", value);
                        Ok(value.evaluate()?)
                    },
                    None => Err(ASTError::VariableNotFound { var: name.clone() }),
                }
            },
            ASTNode::UnaryArithmetic(operator, value) => { // Implementation of Unary Arithmetic Operations
                let val = value.evaluate()?;
                match operator { 
                    ArithmeticOperator::Negate => {
                        match val {
                            ASTConstant::Number(value) => Ok(ASTConstant::Number(-value)),
                            ASTConstant::Array(value) => {
                                let mut arr = Vec::new();
                                for v in value {
                                    if v.get_constant_info().0 == "Number" {
                                        arr.push(ASTConstant::Number(-v.get_value().parse::<f64>().unwrap()));
                                    }else {
                                        arr.push(v.clone());
                                    }
                                }
                                Ok(ASTConstant::Array(arr))
                            },
                            _ => Err(ASTError::InvalidOperation(ArithmeticOperator::Negate.to_string().to_owned(), "bool".to_owned(), "string".to_owned())),
                        }
                    },
                    _ => Err(ASTError::InvalidUnaryOperator),
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
                                    _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                }
                            },
                            ASTConstant::Array(value) => {
                                match operator {
                                    ArithmeticOperator::Add => {
                                        // Add value to all ASTConstants
                                        let arr = value.iter().map(|value| {
                                            let mut element = value.clone();
                                            match &mut element {
                                                ASTConstant::Number(num) => {
                                                    *num += left;
                                                }
                                                ASTConstant::Array(inner_array) => {
                                                    // *inner_array = inner_array.iter().map(|inner| {
                                                        
                                                    // })
                                                    todo!("Add to Nested Array");
                                                }
                                                _ => unreachable!(),
                                            }
                                            element
                                        }).collect();
                                        Ok(ASTConstant::Array(arr))
                                    },
                                    ArithmeticOperator::Subtract => {
                                        todo!("Implement Subtract")
                                    },
                                    ArithmeticOperator::Multiply => {
                                        let arr = value.iter().map(|value| {
                                            let mut element = value.clone();
                                            match &mut element {
                                                ASTConstant::Number(num) => {
                                                    *num *= left;
                                                }
                                                _ => unreachable!(),
                                            }
                                            element
                                        }).collect();
                                        Ok(ASTConstant::Array(arr))
                                    },
                                    _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                }
                            }
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    }
                    ASTConstant::Array(value) => {
                        match right {
                            ASTConstant::Number(right) => {
                                match operator {
                                    ArithmeticOperator::Add => {
                                        let arr = value.iter().map(|value| {
                                        let mut element = value.clone();
                                        match &mut element {
                                            ASTConstant::Number(num) => {
                                                *num += right;
                                            }
                                            ASTConstant::Array(inner_array) => {
                                                // *inner_array = inner_array.iter().map(|inner| {
                                                    
                                                // })
                                                todo!("Add to Nested Array");
                                            }
                                            _ => unreachable!(),
                                        }
                                        element
                                    }).collect();
                                    Ok(ASTConstant::Array(arr))
                                    },
                                    ArithmeticOperator::Subtract => {
                                        let arr = value.iter().map(|value| {
                                            let mut element = value.clone();
                                            match &mut element {
                                                ASTConstant::Number(num) => {
                                                    *num -= right;
                                                }
                                                ASTConstant::Array(inner_array) => {
                                                    todo!("Add to Nested Array");
                                                }
                                                _ => unreachable!(),
                                            }
                                            element
                                        }).collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    ArithmeticOperator::Multiply => {
                                        let arr = value.iter().map(|value| {
                                            let mut element = value.clone();
                                            match &mut element {
                                                ASTConstant::Number(num) => {
                                                    *num *= right;
                                                }
                                                ASTConstant::Array(inner_array) => {
                                                    todo!("Add to Nested Array");
                                                }
                                                _ => unreachable!(),
                                            }
                                            element
                                        }).collect();
                                        Ok(ASTConstant::Array(arr))
                                    },
                                    ArithmeticOperator::Divide => {
                                        let arr = value.iter().map(|value| {
                                            let mut element = value.clone();
                                            match &mut element {
                                                ASTConstant::Number(num) => {
                                                    *num = *num / right;
                                                }
                                                ASTConstant::Array(inner_array) => {
                                                    todo!("Add to Nested Array");
                                                }
                                                _ => unreachable!(),
                                            }
                                            element
                                        }).collect();
                                        Ok(ASTConstant::Array(arr))
                                    },
                                    _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),

                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    }
                    _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                }
            },
            ASTNode::UnaryLogic(operator, value) => {
                let val = value.evaluate()?;
                match val {
                    ASTConstant::Bool(value) => {
                        match operator {
                        LogicOperator::Not => Ok(ASTConstant::Bool(!value)),
                        _ => Err(ASTError::InvalidUnaryOperator),
                        }
                    },
                    ASTConstant::Array(value) => {
                        match operator {
                            LogicOperator::Not => {
                                let arr = value.iter().map(|value| {
                                    let mut element = value.clone();
                                    match &mut element {
                                        ASTConstant::Bool(value) => {
                                            *value = !value.to_string().parse::<bool>().unwrap();
                                        },
                                        _ => unreachable!(),
                                    }
                                    element
                                }).collect();
                                Ok(ASTConstant::Array(arr))
                            }
                            _ => Err(ASTError::InvalidUnaryOperator),
                        }
                    }
                    _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
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
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
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
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    },
                    ASTConstant::String(left) => {
                        match right {
                            ASTConstant::String(right) => {
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    }
                    ASTConstant::Array(left) => {
                        match right {
                            ASTConstant::Number(right) => {
                                match operator {
                                    LogicOperator::Greater => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::Number(num) => *num > right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::Less => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::Number(num) => *num < right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::Number(num) => *num >= right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::Number(num) => *num <= right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::Number(num) => *num == right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::Number(num) => *num != right,
                                        _ => unreachable!(),
                                    }))),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            }
                            ASTConstant::Array(right) => {
                                match operator {
                                    LogicOperator::Equal => {
                                        for i in 0..left.len() {
                                            // println!("{:?}", left[i].get_value());
                                            // println!("{:?}", right[i].get_value());
                                            if left[i].get_value() != right[i].get_value() {
                                                return Ok(ASTConstant::Bool(false));
                                            }
                                            // println!("is equal");
                                        }
                                        Ok(ASTConstant::Bool(true))
                                    },
                                    LogicOperator::NotEqual => {
                                        for i in 0..left.len() {
                                            // println!("{:?}", left[i].get_value());
                                            // println!("{:?}", right[i].get_value());
                                            if left[i].get_value() != right[i].get_value() {
                                                return Ok(ASTConstant::Bool(true));
                                            }
                                            // println!("is equal");
                                        }
                                        Ok(ASTConstant::Bool(false))
                                    },
                                    LogicOperator::Greater | LogicOperator::Less | LogicOperator::GreaterOrEqual | LogicOperator::LessOrEqual => {
                                        todo!("Implement logic operator for array");
                                    }
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            }
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    },
                }
            }
            ASTNode::Function(function_name, args) => {
                match function_name {
                    Functions::Contains => todo!(),
                    Functions::At => {
                        let set = args[0].evaluate()?;
                        let index = args[1].evaluate()?;

                        let idx = index.get_value().parse::<usize>().expect("index must be an integer");

                        match set {
                            ASTConstant::Array(arr)=> {
                                let entry = &arr[idx];
                                match entry {
                                    ASTConstant::Bool(value) => {
                                        Ok(ASTConstant::Bool(*value))
                                    },
                                    ASTConstant::Number(value) => {
                                        Ok(ASTConstant::Number(*value))
                                    },
                                    ASTConstant::String(value) => {
                                        Ok(ASTConstant::String(value.to_string()))
                                    }
                                    ASTConstant::Array(value) => todo!("array at array index"),
                                }
                            },
                            _ => Err(ASTError::InvalidFunctionInvocation("at".to_owned())),
                        }
                        
                    },
                }
            },
            ASTNode::Array(val) => {
                let mut arr = vec![];
                for v in val {
                    arr.push(v.evaluate()?);
                }
                Ok(ASTConstant::Array(arr))
            },
        }
    }

    fn format(&self) -> String {
        match self {
            ASTNode::ConstantBool(value) => value.to_string(),
            ASTNode::ConstantNumber(value) => value.to_string(),
            ASTNode::ConstantString(value) => value.clone(),
            ASTNode::Variable(name, map_ref) => get_var!(value name.as_str()).unwrap(),
            ASTNode::UnaryArithmetic(operator, value) => {
                format!("\t{}\t\n{}", operator.to_string(), value.format())
            },
            ASTNode::BinaryArithmetic(operator, left, right) => {
                format!("\t{}\n{}\t\t{}", operator.to_string(), left.format(), right.format())
            },
            ASTNode::UnaryLogic(operator, value) => {
                format!("\t{}\t\n{}", operator.to_string(), value.format())
            },
            ASTNode::BinaryLogic(operator, left, right) => {
                format!("\t{}\n{}\t\t{}", operator.to_string(), left.format(), right.format())
            }
            ASTNode::Array(values) => format!("{}\n", values.iter().map(|value| value.format()).collect::<Vec<String>>().join("\n")),
            ASTNode::Function(function_name, params) => format!("{}({})", function_name.to_string(), params.iter().map(|value| value.format()).collect::<Vec<String>>().join("\n")),
        }
    }

}

/// Build the tree from a Vec of tokens and return the AST and the root node
fn parse_postfix(tokens: VecDeque<String>) -> Result<(Vec<ASTNode>, ASTNode), ASTError>{
    let mut ast_vec: Vec<ASTNode> = vec![];
    let mut stack: Vec<ASTNode> = vec![];

    for token in tokens {
        if is_operator(token.as_str()) {
            match ArithmeticOperator::from_str(token.as_str()) {
                Ok(value) => {
                    match value {
                        ArithmeticOperator::Negate => {
                            let node = ASTNode::UnaryArithmetic(ArithmeticOperator::Negate, Box::new(stack.pop().unwrap()));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Add => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Add, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Subtract => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Subtract, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Multiply => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Multiply, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Divide => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Divide, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        ArithmeticOperator::Modulo => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(ArithmeticOperator::Modulo, Box::new(left), Box::new(right));
                            ast_vec.push(node.clone());
                            stack.push(node);
                        }
                    }
                },
                Err(_) => {
                    //println!("{} is not an Arithmetic Operator", token);
                    match LogicOperator::from_str(token.as_str()) {
                        Ok(value) => {
                            match value {
                                LogicOperator::Not => {
                                    let node = ASTNode::UnaryLogic(LogicOperator::Not, Box::new(stack.pop().unwrap()));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::And => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::And, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::Or => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::Or, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::Equal => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::Equal, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::NotEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::NotEqual, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::Greater => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::Greater, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::Less => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::Less, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::GreaterOrEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::GreaterOrEqual, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                },
                                LogicOperator::LessOrEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(LogicOperator::LessOrEqual, Box::new(left), Box::new(right));
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                            }
                        },
                        Err(_) => {
                            println!("{} is not an Logic Operator", token);
                        }
                    }
                },
            }
        }else{ // Parse Operand in respective type
            if token.starts_with("$"){ // If the token starts with $ a variable is used

                if token.contains("."){
                    let parts: Vec<&str> = token.split(".").collect();
                    let func: Vec<&str> = str::split(parts[1], "(").collect(); // example: at(1) -> at, 1)
                    let args = Functions::get_args(func[1]);
                    match func[0] {
                        "at" => {
                            if let Some(arg) = args{
                                if arg.len() != 1{
                                    return Err(ASTError::InvalidFunctionParameter("at".to_owned()))
                                }
                                // At expects a number as the index of the array.
                                let node = ASTNode::Function(Functions::At, vec![
                                    Box::new(ASTNode::Variable(parts[0][1..].to_string(), get_variable_map_instance())), // Pointer to variable
                                    Box::new(ASTNode::ConstantNumber(arg[0].clone().parse::<f64>().unwrap()))   // Argument to the function
                                    ]);
                                ast_vec.push(node.clone());
                                stack.push(node);
                                println!("Pushed");
                            }else{
                                return Err(ASTError::InvalidFunctionParameter("at".to_owned()))
                            }
                        },
                        "contains" => {
                            todo!("Implement contains");
                        },
                        _ => {
                            todo!("Implement more functions");
                        }
                    }
                    
                }else {  
                    let node = ASTNode::Variable(token[1..].to_string(), get_variable_map_instance());
                    ast_vec.push(node.clone());
                    stack.push(node);
                }
            }else{
                match token.parse::<f64>() {
                    Ok(value) => {
                        let node = ASTNode::ConstantNumber(value);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    },
                    Err(_) => {
                        //println!("{} is not a number", token);
                        match token.parse::<bool>(){
                            Ok(value) => {
                                let node = ASTNode::ConstantBool(value);
                                ast_vec.push(node.clone());
                                stack.push(node);
                            },
                            Err(_) => {
                                //println!("{} is not a boolean", token);
                                let node: ASTNode = ASTNode::ConstantString(token);
                                ast_vec.push(node.clone());
                                stack.push(node);
                            }
                        }
                    },
                }
            }
        }
    }

    let root = stack.pop().unwrap();

    Ok((ast_vec, root))
}

/// The shunting yard algorithm by Dijkstra transforms the infix logic expression into postfix.
fn shunting_yard_algorithm(tokens: Vec<String>) -> Result<VecDeque<String>, &'static str> {
    let mut stack: Vec<String> = vec![]; // Stack for operators
    let mut output_queue: VecDeque<String> = VecDeque::new();

    for token in tokens.iter() {
        println!("Stack: {:?}", stack);
        println!("Output Queue: {:?}", output_queue);
        if !is_operator(token){
            output_queue.push_back(token.clone());
        }else{
            match token.as_str() {
                "(" => {
                    stack.push(token.to_string());
                },
                ")" => {
                    while stack.last() != Some(&"(".to_owned()) {
                        output_queue.push_back(stack.pop().unwrap());
                        if stack.is_empty() {
                            return Err("Unmatched Parentheses: Empty Stack. No preceeding parentheses");
                        }
                    }
                    stack.pop(); // Remove parenthesis
                },
                _ => {

                    while !stack.is_empty() {
                        if stack.last().unwrap() == "(" {
                            break;
                        }
                        let op_precedence = operator_precedence(stack.last().unwrap());
                        let self_precedence = operator_precedence(token);
                        //println!("op_precedence: {:?}, self_precedence: {:?}", op_precedence, self_precedence);
                        if op_precedence >= self_precedence {
                            output_queue.push_back(stack.pop().unwrap());
                        }else{
                            break;
                        }
                    }
                    stack.push(token.to_string());
                }
            }
        }
    }

    while !stack.is_empty() {
        let val = stack.pop().unwrap();
        if val == "(" {
            return Err("Unmatched Parentheses: Leftover on Stack. More parentheses are opening than closing");
        }
        output_queue.push_back(val);
    }
    println!("Output Queue: {:?}", output_queue);

    Ok(output_queue)
}

/// Return the precedence level of the operator
fn operator_precedence(operator: &str) -> Option<u8> {
    match operator {
        "||" => Some(0),                                // Or
        "&&" => Some(1),                                // And
        "==" | "!=" => Some(2),                         // Equality
        "<" | ">" | "<=" | ">=" => Some(3),             // Comparison
        "+" | "-" => Some(4),                           // Addition, Subtraction
        "*" | "/" | "%" => Some(5),                     // Multiplication, Division, and Modulo
        "!" | "neg" => Some(6),                         // Unary Operators
        "(" | ")" | "[" | "]" | "{" | "}" => Some(7),   // Parentheses and Brackets for Functions and arrays
        _ => None,                                      // No precedence for other operators
    }
}


/// Is the token an Operator
fn is_operator(token: &str) -> bool{
    token == "+" || token == "-" || token == "*" || token == "/" || token == "%"
    || token == "||" || token == "&&" || token == "==" || token == "!=" || token == "!"
    || token == "<" || token == "<=" || token == ">" || token == ">=" || token == "(" 
    || token == ")" || token == "neg"
}

/// Match token for function names and return function name
fn is_function(token: &str) -> Option<Functions>{
    match token {
        "at" => Some(Functions::At),
        "contains" => Some(Functions::Contains),
        _ => None
    }
}

#[derive(Debug, Clone)]
enum Token{
    Number(f64),
    Word(String),
    Bool(bool),
    Operator(String),
    Params(Box<Token>),
    Function(Functions),
    Variable(String),
}

impl Token {
    fn to_string(&self) -> String{
        match self {
            Token::Number(value) => format!("{}", value),
            Token::Word(value) => format!("{}", value),
            Token::Bool(value) => format!("{}", value),
            Token::Operator(value) => format!("{}", value),
            Token::Params(value) => format!("{:?}", value),
            Token::Function(value) => format!("{:?}", value),
            Token::Variable(value) => format!("{}", value),
        }
    }

    fn from_str(token_type: &str, string: &str) -> Result<Self, &'static str>{
        match token_type {
            "NUMBER" => Ok(Token::Number(string.parse::<f64>().unwrap())),
            "BOOL" => Ok(Token::Bool(string.parse::<bool>().unwrap())),
            "OPERATOR" => Ok(Token::Operator(string.to_string())),
            "WORD" => Ok(Token::Word(string.to_string())),
            "FUNCTION" => Ok(Token::Function(is_function(string).unwrap())),
            "VAR" => Ok(Token::Variable(string.to_string())),
            "PARAMS" => Ok(Token::Params(Box::new(Token::from_str("PARAMS", string).unwrap()))),
            _ => Err("Invalid Token"),
        }
    }
}

// /// Tokenizer for input strings into the ast
// fn tokenize(input: &str) -> Vec<Token>{
//     let mut tokens: Vec<Token> = vec![];
    
//     let token_patterns = [
//         ("BOOL", "(true|false)"),
//         ("OPERATOR", r#"\+|\-|\*|\/|\%|neg|\\<|\\>|\\<=|\\>=|\=\=|\!\=|\!"#),
//         ("FUNCTION", r#"\.([^\d]+)\("#),
//         ("VAR", r#"\$(\w*)[^.]"#),
//         ("NUMBER", r#"\d+(\.\d+)?"#),
//         ("WORD", "([a-zA-Z]*)"),
//     ];

//     let mut remaining_input = input;
//     while !remaining_input.is_empty() {
//         let mut matched_token = None;
//         let mut matched_token_length = 0;

//         for &(token_type, pattern) in &token_patterns {
//             println!("Matching pattern {} with {}", token_type, remaining_input);
//             let regex = Regex::new(&format!("{}", pattern)).unwrap();
//             if let Some(matched) = regex.find(remaining_input) {
//                 let token = matched.as_str().to_string();
//                 println!("{}: {}", token_type, token);
//                 let token_length = matched.end();
//                 if token_length > matched_token_length {
//                     matched_token = Some((token_type.to_string(), token));
//                     matched_token_length = token_length;
//                 }
//             }

//             if let Some((_, tok)) = matched_token {
//                 println!("{}: {}", token_type, tok);
//                 let t = match token_type {
//                     "NUMBER" => Token::Number(tok.parse::<f64>().unwrap()),
//                     "BOOL" => Token::Bool(tok.parse::<bool>().unwrap()),
//                     "WORD" => Token::Word(tok.to_string()),
//                     "OPERATOR" => Token::Operator(tok.to_string()),
//                     "FUNCTION" => Token::Function(is_function(&tok).unwrap()),
//                     "VAR" => Token::Variable(tok.to_string()),
//                     "PARAMS" => Token::Params(Box::new(Token::from_str("PARAMS", &tok).unwrap())),
//                     _ => panic!("Invalid Token"),
//                 };
//                 tokens.push(t);
//                 remaining_input = &remaining_input[matched_token_length..];
//             }
//             else{
//                 panic!("Unkown token at position {}", input.len() - remaining_input.len());
//             }
//         }
//     }

//     tokens

// }

/// This macro builds an AST from a string
/// The input is a string with the infix notation separated by spaces
/// The return type is a tuple with the first entry being the full AST and the second entry being the root node
macro_rules! build_ast {
    ($str_pattern:expr) => {    
        parse_postfix(shunting_yard_algorithm($str_pattern.split(" ").map(|s| s.to_string()).collect::<Vec<String>>()).unwrap()).unwrap()
    };
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

#[test]
fn test_ast_macro(){
    let (ast, root) = build_ast!("5 == 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
}

#[test]
fn test_shunting_yard(){
    // 5 + 5 > 17 - (5 - neg 10)
    let tokens = vec!["5".to_owned(), "+".to_owned(), "5".to_owned(), ">".to_owned(), "17".to_owned(), "-".to_owned(), "(".to_owned(), "5".to_owned(), "-".to_owned(), "neg".to_owned(), "10".to_owned(), ")".to_owned()];

    //let tokens: Vec<String> = vec!["5".to_owned(), ">".to_owned(), "(".to_owned(), "6".to_owned(), "+".to_owned(), "5".to_owned(), ")".to_owned()];

    let output = shunting_yard_algorithm(tokens);
    let output = output.unwrap();
    println!("Output: {:?}", output);
    for o in output.iter(){
        print!("{}", o);
    }
    println!("");
    
    let (ast, root) = parse_postfix(output).unwrap();

    println!("Root: {}", root.format());

    let val = root.evaluate().unwrap();
    let (const_type, value) = val.get_constant_info();

    assert_eq!(value, "false");
}

#[test]
fn test_all_operations(){
    // Greater
    let (ast, root) = build_ast!("5 > 4");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Greater: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // Less
    let (ast, root) = build_ast!("3 < 4");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Less: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // GreaterOrEqual
    let (ast, root) = build_ast!("5 >= 4");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("GreaterOrEqual: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // LessOrEqual
    let (ast, root) = build_ast!("4 <= 4");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("LessOrEqual: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // Equal
    let (ast, root) = build_ast!("5 == 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Equal: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");

    // NotEqual
    let (ast, root) = build_ast!("5 != 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("NotEqual: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "false");

    // Not
    let (ast, root) = build_ast!("! true");
    
    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Not: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "false");

    // Add
    let (ast, root) = build_ast!("5.5 + 5.0");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Add: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "10.5");

    // Subtract
    let (ast, root) = build_ast!("5 - 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Subtract: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "0");

    // Multiply
    let (ast, root) = build_ast!("5 * 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Multiply: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "25");

    // Divide
    let (ast, root) = build_ast!("5 / 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Divide: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "1");

    // Modulo
    let (ast, root) = build_ast!("5 % 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Modulo: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "0");

    // Negate
    let (ast, root) = build_ast!("neg 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Negate: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "-5");

}

#[test]
fn test_complex_ast(){
    // Test combination of Arithmetic and Logic Operators
    let (ast, root) = build_ast!("5 + 5 > 7");


    // 5 + 5 > 7
    // 5 -> Output Queue [5] Stack []
    // + -> Output Queue [5] Stack [+]
    // 5 -> Output Queue [5, 5] Stack [+]
    // > -> Output Queue [5, 5, +] Stack [>]
    // 7 -> Output Queue [5, 5, +, 7] Stack [>]
    // [5, 5, +, 7, >]

    // 5 + 5 > 7
    // 5 -> Output Queue [5] Stack []
    // + -> Output Queue [5] Stack [+]
    // 5 -> Output Queue [5, 5] Stack [+]
    // > -> Output Queue [5, 5] Stack [+, >]
    // 7 -> Output Queue [5, 5, 7] Stack [+, >]
    // [5, 5, 7, >, +]


    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Add Greater: {}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}

#[test]
fn test_more_complex_ast(){
    let (ast, root) = build_ast!("( 17 * 3 ) % 10 == 1");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}

#[test]
fn test_variables(){

    let map = get_variable_map_instance();
    set_var!("x", "5");

    let (ast, root) = build_ast!("$x == 5");

    let val = root.evaluate().unwrap();
    let (const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}

#[test]
fn test_str_var(){
    let map = get_variable_map_instance();
    set_var!("x", "airport");

    let (ast, root) = build_ast!("$x == milestone");
    
    
    set_var!("x", "milestone");

    let val = root.evaluate().unwrap();
    let (const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
    assert_eq!(const_type, "Bool");
    assert_eq!(value, "true");
}

#[test]
fn test_function_at(){    
    set_var!("arr", "[0,1,2,3]");

    let (_, root) = build_ast!("$arr.at(1) != 1");

    let val = root.evaluate().unwrap();
    let v = val.get_value();
    println!("{}", v);
    assert_eq!(v, "false");

}

#[test]
fn test_arr() {
    set_var!("arr", "[0,1,2,3]");
    set_var!("arr2", "[0,1,2,3]");
    set_var!("arr3", "[0,1,2,4]");

    let (_, root) = build_ast!("$arr == $arr2");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "true");

    let (_, root) = build_ast!("$arr != $arr3");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "true");

}

// #[test]
// fn test_tokenizer(){
//     let string = "15.0 + 5 > 17 || hallo == hallo && true";

//     // String "15.0 + 5 > 17 || hallo == hallo && true || &arr.at(1) == 1"
//     // Expected tokens: [15.0, +, 5, >, 17, ||, hallo, ==, hallo, &&, true, ||, &arr.at(1), ==, 1]

//     let tokens = tokenize(string);

//     println!("Tokens: {:?}", tokens);
// }

#[test]
fn negate_array() {
    set_var!("arr", "[0,1,2,3]");

    let (_, root) = build_ast!("neg $arr");

    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "[-0,-1,-2,-3]");
}

#[test]
fn test_arrays() {
    set_var!("arr", "[0,1,2,3]");
    set_var!("arr2", "[0,1,2,3]");
    set_var!("arr3", "[0,1,2,4]");

    let (_, root) = build_ast!("$arr + 1");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "[1,2,3,4]");
    assert_eq!(get_var!(value "arr").unwrap(), "[0,1,2,3]");

    let (_, root) = build_ast!("$arr + 5 > 4");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "true");

    let (_ , root) = build_ast!("neg $arr.at(1) == 1");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "false");
}