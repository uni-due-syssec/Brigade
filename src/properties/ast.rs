use core::panic;
use std::collections::VecDeque;
use std::fmt::format;

use crate::{get_var, set_var, utils};

use super::error::{self, ASTError};

use super::environment::{get_variable, VariableMap, get_variable_map_instance, VarValues, GetVar};
use std::str::FromStr;
use ethnum::{u256, i256, AsU256};
use regex::Regex;
use sha3::digest::typenum::U124;

/// This file describes an Abstract Syntax Tree which should contain as leaves constants and the branches refer to logical or arithmetic operators.
/// The AST consists of Nodes see ASTNode struct
/// When evaluating the AST an ASTConstant is returned. See ASTConstant struct


/// The conversion target types for the AST results
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionTarget {
    String,
    Number,
    SignedNumber,
    Hex,
    Unknown(String),
}

impl From<&str> for ConversionTarget {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "string" | "'string'" => ConversionTarget::String,
            "u256" | "'u256'" => ConversionTarget::Number,
            "i256" | "'i256'" => ConversionTarget::SignedNumber,
            "hex" | "'hex'" => ConversionTarget::Hex,
            _ => ConversionTarget::Unknown(s.to_string()),
        }
    }
}

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
    As, // Converts the value to correct representation
}

impl Functions{
    pub fn to_string(&self) -> &str {
        match self {
            Functions::Contains => "contains",
            Functions::At => "at",
            Functions::As => "as",
        }
    }

    pub fn from_str(string: &str) -> Result<Functions, ASTError> {
        match string {
            "contains" => Ok(Functions::Contains),
            "at" => Ok(Functions::At),
            "as" => Ok(Functions::As),
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
    ConstantNumber(u256),
    ConstantSignedNumber(i256),
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
    Number(u256),
    SignedNumber(i256),
    String(String),
    Array(Vec<ASTConstant>),
}

impl ASTConstant{
    pub fn convert(&self, target: ConversionTarget) -> Result<ASTConstant, ASTError> {
        match target {
            ConversionTarget::String => {
                Ok(ASTConstant::String(self.get_value()))
            },
            ConversionTarget::Number => {
                match self {
                    ASTConstant::Number(v) => Ok(ASTConstant::Number(*v)),
                    ASTConstant::SignedNumber(v) => Ok(ASTConstant::Number(v.as_u256())),
                    ASTConstant::String(v) => {
                        if v.starts_with("0x"){
                            Ok(ASTConstant::Number(u256::from_str_hex(v).unwrap()))
                        }else if v.starts_with("u256:") || v.starts_with("i256:"){
                            Ok(ASTConstant::Number(u256::from_str(&v[5..]).unwrap()))
                        }else{
                            let num = v.parse::<u256>();
                            match num{
                                Ok(v) => Ok(ASTConstant::Number(v)),
                                Err(e) =>Err(ASTError::InvalidConversion(v.to_string(), "number".to_string()))
                            }
                        }
                    }
                    _ => Err(ASTError::InvalidConversion(self.get_value().to_string(), "number".to_string()))
                }
            },
            ConversionTarget::SignedNumber => {
                match self {
                    ASTConstant::Number(v) => Ok(ASTConstant::SignedNumber(v.as_i256())),
                    ASTConstant::SignedNumber(v) => Ok(ASTConstant::SignedNumber(*v)),
                    ASTConstant::String(v) => {
                        if v.starts_with("0x"){
                            Ok(ASTConstant::SignedNumber(i256::from_str_hex(v).unwrap()))
                        }else if v.starts_with("u256:") || v.starts_with("i256:"){
                            Ok(ASTConstant::SignedNumber(i256::from_str(&v[5..]).unwrap()))
                        }else{
                            let num = v.parse::<i256>();
                            match num{
                                Ok(v) => Ok(ASTConstant::SignedNumber(v)),
                                Err(e) =>Err(ASTError::InvalidConversion(v.to_string(), "signed number".to_string()))
                            }
                        }
                    }
                    _ => Err(ASTError::InvalidConversion(self.get_value().to_string(), "signed number".to_string()))
                }
            },
            ConversionTarget::Hex => {
                match self {
                    ASTConstant::Number(v) => Ok(ASTConstant::String(format!("0x{:x}", *v))),
                    ASTConstant::SignedNumber(v) => Ok(ASTConstant::String(format!("0x{:x}", *v))),
                    ASTConstant::String(v) => {
                        if v.starts_with("0x"){
                            Ok(ASTConstant::String(v.to_string()))
                        }else{
                            Err(ASTError::InvalidConversion(v.to_string(), "hex".to_string()))
                        }
                    }
                    _ => Err(ASTError::InvalidConversion(self.get_value().to_string(), "hex".to_string()))
                }
            },
            ConversionTarget::Unknown(s) => {
                println!("Unknown conversion target {}", s);
                Err(ASTError::UnknownConversionTarget(s))
            },
        }
    }

    pub fn get_constant_info(&self) -> (&str, String) {
        match self {
            ASTConstant::Bool(value) => ("Bool", value.to_string()),
            ASTConstant::Number(value) => ("Number", value.to_string()),
            ASTConstant::SignedNumber(value) => ("SignedNumber", value.to_string()),
            ASTConstant::String(value) => ("String", value.clone()),
            ASTConstant::Array(value) => ("Array", format!("{:?}", value)),
        }
    }

    pub fn get_value(&self) -> String {
        match self {
            ASTConstant::Bool(value) => value.to_string(),
            ASTConstant::Number(value) => value.to_string(),
            ASTConstant::SignedNumber(value) => value.to_string(),
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
            match value.parse::<u256>() {
                Ok(value) => ASTConstant::Number(value),
                Err(_) => {
                    match value.parse::<bool>() {
                        Ok(value) => ASTConstant::Bool(value),
                        Err(_) => {
                            match  value.parse::<i256>() {
                                Ok(value) => ASTConstant::SignedNumber(value),
                                Err(_) => ASTConstant::String(value),
                            }
                        }
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
            ASTNode::ConstantSignedNumber(value) => Ok(ASTConstant::SignedNumber(*value)),
            ASTNode::ConstantString(value) => Ok(ASTConstant::String(value.clone())),
            ASTNode::Variable(name, map_ref) => {

                match get_variable(map_ref, name) {
                    Some(value) => {
                        // println!("{:?}", value);
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
                            ASTConstant::Number(value) => Ok(ASTConstant::SignedNumber(-value.as_i256())),
                            ASTConstant::SignedNumber(value) => Ok(ASTConstant::SignedNumber(-value)),
                            ASTConstant::Array(value) => {
                                let mut arr = Vec::new();
                                for v in value {
                                    if v.get_constant_info().0 == "Number" {
                                        arr.push(ASTConstant::SignedNumber(-v.get_value().parse::<i256>().unwrap()));
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
                let left_clone = left.clone();
                let right = right.evaluate()?;
                match left {
                    ASTConstant::String(l) => {
                        match right {
                            ASTConstant::Number(r) => {
                                let val = left_clone.convert(ConversionTarget::Number).unwrap();
                                let val_node = ASTNode::ConstantNumber(u256::from_str(val.get_value().as_str()).unwrap());
                                ASTNode::BinaryArithmetic(operator.clone(), Box::new(val_node), Box::new(ASTNode::ConstantNumber(r))).evaluate()
                            },
                            ASTConstant::SignedNumber(r) => {
                                let val = left_clone.convert(ConversionTarget::SignedNumber).unwrap();
                                let val_node = ASTNode::ConstantSignedNumber(i256::from_str(val.get_value().as_str()).unwrap());
                                ASTNode::BinaryArithmetic(operator.clone(), Box::new(val_node), Box::new(ASTNode::ConstantSignedNumber(r))).evaluate()
                            },
                            _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                        }
                    }
                    ASTConstant::SignedNumber(left) => {
                        match right {
                            ASTConstant::SignedNumber(right) => {
                                match operator {
                                    ArithmeticOperator::Add => Ok(ASTConstant::SignedNumber(left + right)),
                                    ArithmeticOperator::Subtract => Ok(ASTConstant::SignedNumber(left - right)),
                                    ArithmeticOperator::Multiply => Ok(ASTConstant::SignedNumber(left * right)),
                                    ArithmeticOperator::Divide => Ok(ASTConstant::SignedNumber(left / right)),
                                    ArithmeticOperator::Modulo => Ok(ASTConstant::SignedNumber(left % right)),
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
                                                ASTConstant::SignedNumber(num) => {
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
                                                    ASTConstant::SignedNumber(num) => {
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
                                },
                            ASTConstant::String(value) => {
                                // Try String Conversion
                                if value.starts_with("0x"){
                                    let v = utils::hex_string_to_u256(&value[2..]);
                                    match operator {
                                        ArithmeticOperator::Add => Ok(ASTConstant::SignedNumber(left + v.as_i256())),
                                        ArithmeticOperator::Subtract => Ok(ASTConstant::SignedNumber(left - v.as_i256())),
                                        ArithmeticOperator::Multiply => Ok(ASTConstant::SignedNumber(left * v.as_i256())),
                                        ArithmeticOperator::Divide => Ok(ASTConstant::SignedNumber(left / v.as_i256())),
                                        ArithmeticOperator::Modulo => Ok(ASTConstant::SignedNumber(left % v.as_i256())),
                                        _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                    }
                                }else if value.starts_with("u256:"){
                                    let v = u256::from_str(&value[5..]).unwrap();
                                    match operator {
                                        ArithmeticOperator::Add => Ok(ASTConstant::SignedNumber(left + v.as_i256())),
                                        ArithmeticOperator::Subtract => Ok(ASTConstant::SignedNumber(left - v.as_i256())),
                                        ArithmeticOperator::Multiply => Ok(ASTConstant::SignedNumber(left * v.as_i256())),
                                        ArithmeticOperator::Divide => Ok(ASTConstant::SignedNumber(left / v.as_i256())),
                                        ArithmeticOperator::Modulo => Ok(ASTConstant::SignedNumber(left % v.as_i256())),
                                        _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                    }
                                }else if value.starts_with("i256:"){
                                    let v = i256::from_str(&value[5..]).unwrap();
                                    match operator {
                                        ArithmeticOperator::Add => Ok(ASTConstant::SignedNumber(left + v)),
                                        ArithmeticOperator::Subtract => Ok(ASTConstant::SignedNumber(left - v)),
                                        ArithmeticOperator::Multiply => Ok(ASTConstant::SignedNumber(left * v)),
                                        ArithmeticOperator::Divide => Ok(ASTConstant::SignedNumber(left / v)),
                                        ArithmeticOperator::Modulo => Ok(ASTConstant::SignedNumber(left % v)),
                                        _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                    }
                                }else{
                                    Err(ASTError::InvalidConversion(value, "Number".to_string()))
                                }
                            }
                            _ => Err(ASTError::InvalidBinaryOperator),
                            }
                        },
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
                            ASTConstant::SignedNumber(value) => {
                                if value >= 0 {
                                    let v = value.as_u256();
                                    match operator {
                                        ArithmeticOperator::Add => Ok(ASTConstant::Number(left + v)),
                                        ArithmeticOperator::Subtract => Ok(ASTConstant::Number(left - v)),
                                        ArithmeticOperator::Multiply => Ok(ASTConstant::Number(left * v)),
                                        ArithmeticOperator::Divide => Ok(ASTConstant::Number(left / v)),
                                        ArithmeticOperator::Modulo => Ok(ASTConstant::Number(left % v)),
                                        _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                    }
                                }else{
                                    match operator {
                                        ArithmeticOperator::Add => Ok(ASTConstant::SignedNumber(left.as_i256() + value)),
                                        ArithmeticOperator::Subtract => Ok(ASTConstant::SignedNumber(left.as_i256() - value)),
                                        ArithmeticOperator::Multiply => Ok(ASTConstant::SignedNumber(left.as_i256() * value)),
                                        ArithmeticOperator::Divide => Ok(ASTConstant::SignedNumber(left.as_i256() / value)),
                                        ArithmeticOperator::Modulo => Ok(ASTConstant::SignedNumber(left.as_i256() % value)),
                                        _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                    }
                                }
                            }
                            ASTConstant::String(value) => {
                                // Try String Conversion
                                if value.starts_with("0x"){
                                    let v = utils::hex_string_to_u256(&value[2..]);
                                    match operator {
                                        ArithmeticOperator::Add => Ok(ASTConstant::Number(left + v)),
                                        ArithmeticOperator::Subtract => Ok(ASTConstant::Number(left - v)),
                                        ArithmeticOperator::Multiply => Ok(ASTConstant::Number(left * v)),
                                        ArithmeticOperator::Divide => Ok(ASTConstant::Number(left / v)),
                                        ArithmeticOperator::Modulo => Ok(ASTConstant::Number(left % v)),
                                        _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                    }
                                }else if value.starts_with("u256:"){
                                    let v = u256::from_str(&value[5..]).unwrap();
                                    match operator {
                                        ArithmeticOperator::Add => Ok(ASTConstant::Number(left + v)),
                                        ArithmeticOperator::Subtract => Ok(ASTConstant::Number(left - v)),
                                        ArithmeticOperator::Multiply => Ok(ASTConstant::Number(left * v)),
                                        ArithmeticOperator::Divide => Ok(ASTConstant::Number(left / v)),
                                        ArithmeticOperator::Modulo => Ok(ASTConstant::Number(left % v)),
                                        _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                    }
                                }else if value.starts_with("i256:"){
                                    let v = i256::from_str(&value[5..]).unwrap();
                                    match operator {
                                        ArithmeticOperator::Add => Ok(ASTConstant::SignedNumber(left.as_i256() + v)),
                                        ArithmeticOperator::Subtract => Ok(ASTConstant::SignedNumber(left.as_i256() - v)),
                                        ArithmeticOperator::Multiply => Ok(ASTConstant::SignedNumber(left.as_i256() * v)),
                                        ArithmeticOperator::Divide => Ok(ASTConstant::SignedNumber(left.as_i256() / v)),
                                        ArithmeticOperator::Modulo => Ok(ASTConstant::SignedNumber(left.as_i256() % v)),
                                        _ => Err(ASTError::InvalidArithmeticOperator(operator.to_string().to_owned())),
                                    }
                                }else{
                                    Err(ASTError::InvalidConversion(value, "Number".to_string()))
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
                    ASTConstant::SignedNumber(left) => {
                        match right {
                            ASTConstant::Number(right) => {
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right.as_i256())),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right.as_i256())),
                                    LogicOperator::Greater => Ok(ASTConstant::Bool(left > right.as_i256())),
                                    LogicOperator::Less => Ok(ASTConstant::Bool(left < right.as_i256())),
                                    LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left >= right.as_i256())),
                                    LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left <= right.as_i256())),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            },
                            ASTConstant::SignedNumber(right) => {
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
                            ASTConstant::Array(right) => {
                                todo!("Implement Array Logic with signed numbers")
                            },
                            ASTConstant::String(right) => {
                                if right.starts_with("0x"){
                                    let v = i256::from_str_hex(&right).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left >= v)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else if right.starts_with("u256:"){
                                    let v = u256::from_str(&right[5..]).unwrap().as_i256();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left >= v)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else if right.starts_with("i256:"){
                                    let v = i256::from_str(&right[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left >= v)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else {
                                    Err(ASTError::InvalidConversion(right, "Number".to_string()))
                                }
                            }
                            _ => Err(ASTError::InvalidBinaryOperator),
                        }
                    }
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
                            ASTConstant::SignedNumber(right) => {
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left.as_i256() == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left.as_i256() != right)),
                                    LogicOperator::Greater => Ok(ASTConstant::Bool(left.as_i256() > right)),
                                    LogicOperator::Less => Ok(ASTConstant::Bool(left.as_i256() < right)),
                                    LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left.as_i256() >= right)),
                                    LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left.as_i256() <= right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            },
                            ASTConstant::String(right) => {
                                if right.starts_with("0x"){
                                    let v = u256::from_str_hex(&right).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left >= v)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else if right.starts_with("u256:"){
                                    let v = u256::from_str(&right[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left >= v)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else if right.starts_with("i256:"){
                                    let v = i256::from_str(&right[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left.as_i256() == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left.as_i256() != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left.as_i256() > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left.as_i256() < v)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left.as_i256() >= v)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left.as_i256() <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else{
                                    Err(ASTError::InvalidConversion(right, "Number".to_string()))
                                }
                            }
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
                            ASTConstant::Number(right) => {
                                if left.starts_with("0x"){
                                    let l = u256::from_str_hex(&left).unwrap();
                                    match operator{
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(l >= right)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else if left.starts_with("u256:"){
                                    let l = u256::from_str(&left[5..]).unwrap();
                                    match operator{
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(l >= right)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else if left.starts_with("i256:"){
                                    let l = i256::from_str(&left[5..]).unwrap();
                                    match operator{
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right.as_i256())),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(l != right.as_i256())),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right.as_i256())),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right.as_i256())),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(l >= right.as_i256())),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(l <= right.as_i256())),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else{
                                    Err(ASTError::InvalidConversion(left, "Number".to_string()))
                                }
                            },
                            ASTConstant::SignedNumber(right) => {
                                if left.starts_with("0x"){
                                    let l = i256::from_str_hex(&left).unwrap();
                                    match operator{
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(l >= right)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else if left.starts_with("u256:"){
                                    let l = i256::from_str(&left[5..]).unwrap();
                                    match operator{
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(l >= right)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else if left.starts_with("i256:"){
                                    let l = i256::from_str(&left[5..]).unwrap();
                                    match operator{
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(l >= right)),
                                        LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                }else{
                                    Err(ASTError::InvalidConversion(left, "Number".to_string()))
                                }
                            },
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    }
                    ASTConstant::Array(left) => {
                        match right {
                            ASTConstant::SignedNumber(right) => {
                                match operator {
                                    LogicOperator::Greater => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::SignedNumber(num) => *num > right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::Less => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::SignedNumber(num) => *num < right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::GreaterOrEqual => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::SignedNumber(num) => *num >= right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::LessOrEqual => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::SignedNumber(num) => *num <= right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::SignedNumber(num) => *num == right,
                                        _ => unreachable!(),
                                    }))),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left.iter().all(|element| match element {
                                        ASTConstant::SignedNumber(num) => *num != right,
                                        _ => unreachable!(),
                                    }))),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            }
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
                    Functions::Contains => {
                        let set = args[0].evaluate()?;
                        let value = args[1].evaluate()?;
                        match set {
                            ASTConstant::Array(arr)=> {
                                Ok(ASTConstant::Bool(arr.iter().any(|element| element.get_value() == value.get_value())))
                            },
                            _ => Err(ASTError::InvalidFunctionInvocation("contains".to_owned())),
                        }
                    },
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
                                    ASTConstant::SignedNumber(value) => {
                                        Ok(ASTConstant::SignedNumber(*value))
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
                    Functions::As => {
                        let me = args[0].evaluate()?;
                        let type_name = args[1].evaluate()?;
                        
                        // println!("Type name: {}", type_name.get_value());
                        // println!("Me: {}", me.get_value());

                        let conv = ConversionTarget::from(type_name.get_value().as_str());

                        let converted = me.convert(conv);

                        match converted {
                            Ok(c) => Ok(c),
                            Err(e) => {
                                println!("Conversion failed: {}", e);
                                Ok(me)
                            }
                        }
                    }
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
            ASTNode::ConstantSignedNumber(value) => value.to_string(),
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

impl From<ASTConstant> for ASTNode {
    fn from(value: ASTConstant) -> Self {
        match value {
            ASTConstant::Bool(value) => ASTNode::ConstantBool(value),
            ASTConstant::Number(value) => ASTNode::ConstantNumber(value),
            ASTConstant::SignedNumber(value) => ASTNode::ConstantSignedNumber(value),
            ASTConstant::String(value) => ASTNode::ConstantString(value),
            ASTConstant::Array(value) => {
                let v = value.iter().map(|x| Box::new(ASTNode::from(x.clone()))).collect();
                ASTNode::Array(v)
            },
        }
    }
}


/// Build the tree from a Vec of tokens and return the AST and the root node
pub fn parse_postfix(tokens: VecDeque<String>) -> Result<(Vec<ASTNode>, ASTNode), ASTError>{
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
                            println!("{} is not a Logic Operator", token);
                        }
                    }
                },
            }
        }else{ // Parse Operand in respective type
            if token.starts_with("$"){ // If the token starts with $ a variable is used

                // Variable Function calls
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
                                // At expects a positive number or zero as the index of the array.
                                let node = ASTNode::Function(Functions::At, vec![
                                    Box::new(ASTNode::Variable(parts[0][1..].to_string(), get_variable_map_instance())), // Pointer to variable
                                    Box::new(ASTNode::ConstantNumber(arg[0].clone().parse::<u256>().unwrap()))   // Argument to the function
                                    ]);
                                ast_vec.push(node.clone());
                                stack.push(node);
                            }else{
                                return Err(ASTError::InvalidFunctionParameter("at".to_owned()))
                            }
                        },
                        "contains" => {
                            if let Some(arg) = args{
                                if arg.len() != 1{
                                    return Err(ASTError::InvalidFunctionParameter("contains".to_owned()))
                                }
                                let node = ASTNode::Function(Functions::Contains, vec![
                                    Box::new(ASTNode::Variable(parts[0][1..].to_string(), get_variable_map_instance())), // Pointer to variable
                                    Box::new(ASTNode::ConstantNumber(arg[0].clone().parse::<u256>().unwrap()))   // Argument to the function
                                    ]);
                                ast_vec.push(node.clone());
                                stack.push(node);
                            }else {
                                return Err(ASTError::InvalidFunctionParameter("contains".to_owned()))
                            }
                        },
                        "as" => {
                            if let Some(arg) = args{
                                if arg.len() != 1{
                                    return Err(ASTError::InvalidFunctionParameter("as".to_owned()))
                                }
                                let node = ASTNode::Function(Functions::As, vec![
                                    Box::new(ASTNode::Variable(parts[0][1..].to_string(), get_variable_map_instance())), // Pointer to variable
                                    Box::new(ASTNode::ConstantString(arg[0].clone()))   // Argument to the function
                                ]);
                                ast_vec.push(node.clone());
                                stack.push(node);
                            }else{
                                return Err(ASTError::InvalidFunctionParameter("as".to_owned()))
                            }
                        }
                        _ => {
                            todo!("Implement more functions");
                        }
                    }
                    
                }else {
                    // Parse Variables  
                    let node = ASTNode::Variable(token[1..].to_string(), get_variable_map_instance());
                    ast_vec.push(node.clone());
                    stack.push(node);
                }
            }else{
                // Function calls on all AST Nodes
                if token.contains(".") {
                    let parts: Vec<&str> = token.split(".").collect();
                    let func: Vec<&str> = str::split(parts[1], "(").collect(); // example: at(1) -> at, 1)
                    let args = Functions::get_args(func[1]);
                    match func[0] {
                        "as" => {
                            if let Some(arg) = args{
                                if arg.len() != 1{
                                    return Err(ASTError::InvalidFunctionParameter("as".to_owned()))
                                }
                                // At expects a positive number or zero as the index of the array.
                                let node = ASTNode::Function(Functions::As, vec![
                                    Box::new(parse_token(parts[0].to_string()).expect("Could not parse token")), // Pointer to variable
                                    Box::new(ASTNode::ConstantString(arg[0].clone()))   // Argument to the function
                                    ]);
                                ast_vec.push(node.clone());
                                stack.push(node);
                            }else{
                                return Err(ASTError::InvalidFunctionParameter("as".to_owned()))
                            }
                        },
                        _ => {
                            todo!("Implement more functions");
                        }
                    }
                }
                else{
                    // Parse normal token
                    let node = match parse_token(token){
                        Ok(node) => {
                            ast_vec.push(node.clone());
                            stack.push(node);
                        },
                        Err(e) => {
                            panic!("Error at token parsing {}", e);
                        }
                    };    
                }
                
            }
        }
    }

    let root = stack.pop().unwrap();

    Ok((ast_vec, root))
}

pub fn parse_token(token: String) -> Result<ASTNode, &'static str>{
    match token.parse::<u256>() {
        Ok(value) => {
            Ok(ASTNode::ConstantNumber(value))
        },
        Err(_) => {
            //println!("{} is not a number", token);
            match token.parse::<bool>(){
                Ok(value) => {
                    Ok(ASTNode::ConstantBool(value))
                },
                Err(_) => {

                    match token.parse::<i256>(){
                        Ok(value) => {
                            Ok(ASTNode::ConstantSignedNumber(value))
                        }
                        Err(_) => {
                            //println!("{} is not a boolean", token);
                            Ok(ASTNode::ConstantString(token))
                        }
                    }

                }
            }
        },
    }
}

/// The shunting yard algorithm by Dijkstra transforms the infix logic expression into postfix.
pub fn shunting_yard_algorithm(tokens: Vec<String>) -> Result<VecDeque<String>, &'static str> {
    let mut stack: Vec<String> = vec![]; // Stack for operators
    let mut output_queue: VecDeque<String> = VecDeque::new();

    for token in tokens.iter() {
        // println!("Stack: {:?}", stack);
        // println!("Output Queue: {:?}", output_queue);
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
    // println!("Output Queue: {:?}", output_queue);

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
#[macro_export]
macro_rules! build_ast {
    ($str_pattern:expr) => {
        match parse_postfix(shunting_yard_algorithm($str_pattern.split(" ").map(|s| s.to_string()).collect::<Vec<String>>()).unwrap()){
            Ok((ast, root)) => {
                (ast, root)
            },
            Err(e) => {
                panic!("{}", e);
            }
        }
    };
}


#[test]
fn test_ast(){
    // Example: 5 + 5 > 17 - 15

    let exp_a = ASTNode::ConstantNumber(5.as_u256());
    let exp_b = ASTNode::ConstantNumber(5.as_u256());
    let exp_a_b = ASTNode::BinaryArithmetic(ArithmeticOperator::Add, Box::new(exp_a), Box::new(exp_b));

    let exp_c = ASTNode::ConstantNumber(17.as_u256());
    let exp_d = ASTNode::ConstantNumber(15.as_u256());
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
    
    let (ast, root) = parse_postfix(output).unwrap();

    println!("Root: {}", root.format());

    let val = root.evaluate().unwrap();
    let (const_type, value) = val.get_constant_info();

    assert_eq!(value, "true");
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
    let (ast, root) = build_ast!("5 + 5");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("Add: {}: {}", const_type, value);
    assert_eq!(const_type, "Number");
    assert_eq!(value, "10");

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
    assert_eq!(const_type, "SignedNumber");
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
    set_var!("arr4", "['hello','user','a',5]");

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

    println!("{:?}", get_var!("arr4").unwrap());

}

#[test]
fn convert_values(){
    set_var!("a", "0xff");

    let (_, root) = build_ast!("$a == 255");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "true");

    let (_, root) = build_ast!("255 == $a");
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
    assert_eq!(ret, "[0,-1,-2,-3]");
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

#[test]
fn test_contains() {
    set_var!("arr", "[0,1,2,3]");

    let (_, root) = build_ast!("$arr.contains(1)");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "true");

    let (_, root) = build_ast!("$arr.contains(60.0)");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "false");
}

#[test]
fn test_conversion() {
    let (_, root) = build_ast!("5.as('hex')");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "0x5");

    // Output: ["0x5f", "0x5", "-", ".as(u256)"]
    // Should not work
    let (_, root) = build_ast!("( 0x5f - 0x5 ).as('u256')");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "90");

}

#[test]
fn test_conversion2() {
    set_var!("a", "255");
    let (_, root) = build_ast!("$a.as(hex)");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "0xff");
}

#[test]
fn test_string_arithmetic(){
    let (_, root) = build_ast!("0xff.as(u256) - 1");

    let val = root.evaluate().unwrap();
    let(const_type, value) = val.get_constant_info();
    println!("{}: {}", const_type, value);
}

#[test]
fn test_variable_conversion(){
    set_var!("a", "255");
    let (_, root) = build_ast!("( $a.as(hex) - 1 )");
    let val = root.evaluate().unwrap();
    let ret = val.get_value();
    println!("{}", ret);
    assert_eq!(ret, "0xff");

}