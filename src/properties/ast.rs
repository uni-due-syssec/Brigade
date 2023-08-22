use core::panic;
use std::boxed;
use std::collections::VecDeque;

use crate::{get_var, set_var, utils};

use super::error::ASTError;

use super::environment::{get_variable, VariableMap, get_variable_map_instance, VarValues, GetVar};
use std::str::FromStr;
use ethnum::{u256, i256, AsU256};
use sha3::Digest;

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
    Contains, // Returns true if ASTNode Array contains a ASTNode value contains(value)
    At, // Returns the ASTNode value at index at(index)
    As, // Converts the value to correct representation as(type)
    Slice, // Slice Strings and return a String with character from to slice(start inclusive, end exclusive)
    Push, // Push value to the end of the keystore
    Pop, // Pop value from the end of the keystore
    Keccak256, // Keccak256 Hash
}

impl Functions{
    pub fn to_string(&self) -> &str {
        match self {
            Functions::Contains => "contains",
            Functions::At => "at",
            Functions::As => "as",
            Functions::Slice => "slice",
            Functions::Push => "push",
            Functions::Pop => "pop",
            Functions::Keccak256 => "keccak256",
        }
    }

    pub fn from_str(string: &str) -> Result<Functions, ASTError> {
        match string {
            "contains" => Ok(Functions::Contains),
            "at" => Ok(Functions::At),
            "as" => Ok(Functions::As),
            "slice" => Ok(Functions::Slice),
            "push" => Ok(Functions::Push),
            "pop" => Ok(Functions::Pop),
            "keccak256" => Ok(Functions::Keccak256),
            _ => Err(ASTError::InvalidFunction(string.to_owned())),
        }
    }

    pub fn get_args(string: &str) -> Option<Vec<String>> {
        let s = string[0..string.len()-1].to_owned();
        Some(s.split(",").map(|s| s.trim().to_string()).collect())
    }
}

#[test]
fn test_args(){
    println!("{:?}", Functions::get_args("1, 222)"));
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

impl From<String> for ASTNode {
    fn from(s: String) -> Self {
        if s.starts_with('$'){
            return ASTNode::Variable(s[1..].to_owned(), get_variable_map_instance());
        }
        ASTNode::ConstantString(s)
    }
}

impl From<&str> for ASTNode {
    fn from(s: &str) -> Self {
        if s.starts_with("$"){
            return ASTNode::Variable(s[1..].to_owned(), get_variable_map_instance());
        }
        ASTNode::ConstantString(s.to_owned())
    }
}

impl From<u256> for ASTNode {
    fn from(s: u256) -> Self {
        ASTNode::ConstantNumber(s)
    }
}

impl From<i256> for ASTNode {
    fn from(s: i256) -> Self {
        ASTNode::ConstantSignedNumber(s)
    }
}

impl From<bool> for ASTNode {
    fn from(s: bool) -> Self {
        ASTNode::ConstantBool(s)
    }
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
                            // Add the prefix to already correct hex_strings. Check if correct hex number
                            match u256::from_str_radix(v, 16){
                                Ok(v) => Ok(ASTConstant::String(format!("0x{:x}", v))),
                                Err(e) =>Err(ASTError::InvalidConversion(v.to_string(), "hex".to_string()))
                            }
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

    pub fn print(&self, prefix: &str) {
        match self {
            ASTNode::ConstantBool(b) => println!("{}└── Bool: {}", prefix, b),
            ASTNode::ConstantNumber(n) => println!("{}└── Number: {}", prefix, n),
            ASTNode::ConstantSignedNumber(n) => println!("{}└── SignedNumber: {}", prefix, n),
            ASTNode::ConstantString(s) => println!("{}└── String: {}", prefix, s),
            ASTNode::Array(arr) => {
                println!("{}└── Array:", prefix);
                let last = arr.len() - 1;
                for (i, v) in arr.iter().enumerate() {
                    let new_prefix = if i == last { "   " } else { "│  " };
                    // println!("{}{}", prefix, new_prefix);
                    v.print(&format!("{}{}", prefix, new_prefix));
                }
            },
            ASTNode::Variable(name, _) =>{
                if let Some(v) = get_var!(name){
                    println!("{}└── Variable: {}", prefix, v.get_value())
                }else{
                    println!("{}└── Variable: {}", prefix, name)
                }
            },
            ASTNode::UnaryArithmetic(operator, value) => {
                println!("{}└── Arithmetic: {}", prefix, operator.to_string());
                value.print(&format!("{}    ", prefix));
            },
            ASTNode::BinaryArithmetic(operator, left, right) => {
                println!("{}└── Arithmetic: {}", prefix, operator.to_string());
                left.print(&format!("{}│   ", prefix));
                right.print(&format!("{}    ", prefix));
            },
            ASTNode::UnaryLogic(operator, value) => {
                println!("{}└── Logic: {}", prefix, operator.to_string());
                value.print(&format!("{}    ", prefix));
            },
            ASTNode::BinaryLogic(operator, left, right) => {
                println!("{}└── Logic: {}", prefix, operator.to_string());
                left.print(&format!("{}│   ", prefix));
                right.print(&format!("{}    ", prefix));
            },
            ASTNode::Function(func, args) => {
                println!("{}└── Function: {}", prefix, func.to_string());
                let last = args.len() - 1;
                for (i, arg) in args.iter().enumerate() {
                    let new_prefix = if i == last { "   " } else { "│  " };
                    //println!("{}{}", prefix, new_prefix);
                    arg.print(&format!("{}{}", prefix, new_prefix));
                }
            }
        }
    }

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
                            ASTConstant::Number(_) | ASTConstant::String(_) | ASTConstant::SignedNumber(_)=> {
                                match operator {
                                    LogicOperator::And => Ok(ASTConstant::Bool(left)),
                                    LogicOperator::Or => Ok(ASTConstant::Bool(left)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                    
                                }
                            }
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
                    },
                    Functions::Slice => {
                        let me = args[0].evaluate()?;
                        let start = args[1].evaluate()?;
                        let end = args[2].evaluate()?;

                        let start_index = start.get_value().parse::<usize>().expect("start index must be an integer");
                        let end_index = end.get_value().parse::<usize>().expect("end index must be an integer");

                        match me {
                            ASTConstant::Array(arr) => {
                                Ok(ASTConstant::Array(arr[start_index..end_index].to_vec()))
                            },
                            ASTConstant::String(s) => {
                                if end_index > s.len() {
                                    return Err(ASTError::InvalidSlice(s.clone(), start_index, end_index, s.len()));
                                }
                                Ok(ASTConstant::String(s[start_index..end_index].to_string()))
                            }
                            _ => Err(ASTError::InvalidFunctionInvocation("slice".to_owned())),
                        }

                    },
                    Functions::Push => {
                        let node = args[0].clone();
                        let me = args[0].evaluate()?;
                        let value = args[1].evaluate()?;
                        match me {
                            ASTConstant::Array(arr) => {
                                if let ASTNode::Variable(name, map) = *node {
                                    if let Some(a) = get_var!(&name){
                                        match a{
                                            VarValues::Array(mut inner) => {

                                                match value{
                                                    ASTConstant::Array(arr) => {
                                                        for item in arr{
                                                            inner.push(VarValues::from(item));
                                                        }
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    },
                                                    ASTConstant::Bool(v) => {
                                                        inner.push(VarValues::from(v));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    },
                                                    ASTConstant::Number(v) => {
                                                        inner.push(VarValues::from(v));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    },
                                                    ASTConstant::SignedNumber(v) => {
                                                        inner.push(VarValues::from(v));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    },
                                                    ASTConstant::String(v) => {
                                                        inner.push(VarValues::from(v));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    }
                                                }
                                            },
                                            _ => {
                                                return Err(ASTError::InvalidFunctionInvocation("push".to_owned()));
                                            }
                                        }
                                    }else{
                                        println!("Variable not found: {}", name);
                                        // Build new Array and push
                                        match value {
                                            ASTConstant::Bool(v) => {
                                                let new_arr: Vec<VarValues> = vec![VarValues::from(v)];
                                                set_var!(name, new_arr);
                                                return Ok(ASTConstant::Bool(true));
                                            },
                                            ASTConstant::Number(v) => {
                                                let new_arr: Vec<VarValues> = vec![VarValues::from(v)];
                                                set_var!(name, new_arr);
                                                return Ok(ASTConstant::Bool(true));
                                            },
                                            ASTConstant::SignedNumber(v) => {
                                                let new_arr: Vec<VarValues> = vec![VarValues::from(v)];
                                                set_var!(name, new_arr);
                                                return Ok(ASTConstant::Bool(true));
                                            },
                                            ASTConstant::String(v) => {
                                                let new_arr: Vec<VarValues> = vec![VarValues::from(v)];
                                                set_var!(name, new_arr);
                                                return Ok(ASTConstant::Bool(true));
                                            }
                                            _ => {
                                                return Err(ASTError::InvalidFunctionInvocation("push".to_owned()));
                                            }
                                        }
                                    }
                                }else{
                                    Err(ASTError::InvalidFunctionInvocation("push".to_owned()))
                                }
                            },
                            ASTConstant::String(s) => {
                                let new_string = format!("{}{}", s, value.get_value());
                                Ok(ASTConstant::String(new_string))
                            },
                            _ => {
                                return Err(ASTError::InvalidFunctionInvocation("push".to_owned()));
                            }
                        }
                    },
                    Functions::Pop => {
                        let me = args[0].clone().evaluate()?;
                        match me {
                            ASTConstant::Array(mut arr) => {
                                
                                let last = arr.pop().unwrap();
                                if let ASTNode::Variable(name, _) = *args[0].clone() {
                                    set_var!(name, arr);    
                                }
                                
                                match last {
                                    ASTConstant::Bool(v) => Ok(ASTConstant::Bool(v)),
                                    ASTConstant::Number(v) => Ok(ASTConstant::Number(v)),
                                    ASTConstant::SignedNumber(v) => Ok(ASTConstant::SignedNumber(v)),
                                    ASTConstant::String(v) => Ok(ASTConstant::String(v)),
                                    _ => Err(ASTError::InvalidFunctionInvocation("pop".to_owned())),
                                }

                            },
                            _ => Err(ASTError::InvalidFunctionInvocation("pop".to_owned())),
                        }
                    },
                    Functions::Keccak256 => {
                        let evalled_args = args.iter().map(|x| x.evaluate().unwrap()).collect::<Vec<ASTConstant>>();

                        let serialized_values = encode_packed(&evalled_args).unwrap();

                        let concatenated_bytes = serialized_values.as_slice();

                        let mut hasher = sha3::Keccak256::digest(concatenated_bytes).to_vec();
                        let hex_string = hasher.iter().map(|&num| format!("{:02x}",num)).collect::<Vec<String>>().join("");
                        let s = "0x".to_string() + &hex_string;
                        println!("Keccak256: {}", s);
                        Ok(ASTConstant::String(s))

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

    // Array Helper
    let mut arr: Vec<Box<ASTNode>> = vec![];
    let mut is_array = false;


    let mut skip_next = 0;
    for (id, token) in tokens.iter().enumerate() {
        if skip_next > 0 { // Skip already used tokens from functions
            skip_next -= 1;
            continue;
        }
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

            if let Ok(func) = Functions::from_str(token.as_str()){
                // Parse Functions
                match func {
                    Functions::As => {
                        // As takes just one argument and the preceeding token
                        let args = tokens[id+1].clone();
                        skip_next += 1; // Skip next token
                        let me = stack.pop().unwrap_or(ASTNode::ConstantString("".to_owned()));
                        let node = ASTNode::Function(Functions::As, vec![ Box::new(me) ,Box::new(ASTNode::ConstantString(args))]);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    }
                    Functions::Contains => {
                        // Contains takes one argument and the preceeding token
                        let args = tokens[id+1].clone();
                        skip_next += 1; // Skip next token
                        let me = stack.pop().unwrap_or(ASTNode::ConstantString("".to_owned()));
                        let node = ASTNode::Function(Functions::Contains, vec![ Box::new(me) ,Box::new(ASTNode::ConstantString(args))]);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    },
                    Functions::At => {
                        // At takes one argument and the preceeding token
                        let args = tokens[id+1].clone();
                        skip_next += 1; // Skip next token
                        let me = stack.pop().unwrap_or(ASTNode::ConstantString("".to_owned()));
                        let node = ASTNode::Function(Functions::At, vec![ Box::new(me) ,Box::new(ASTNode::ConstantString(args))]);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    },
                    Functions::Slice => {
                        // Slice takes two arguments and the preceeding token
                        let args = tokens[id+1].clone();
                        let args2 = tokens[id+2].clone();
                        skip_next += 2;
                        let me = stack.pop().unwrap_or(ASTNode::ConstantString("".to_owned()));
                        let node = ASTNode::Function(Functions::Slice, vec![ Box::new(me) ,Box::new(ASTNode::ConstantString(args)), Box::new(ASTNode::ConstantString(args2))]);
                        ast_vec.push(node.clone());
                        stack.push(node);

                    },
                    Functions::Push => {
                        // Push takes one argument and the preceeding token
                        let args = tokens[id+1].clone();
                        skip_next += 1;
                        let me = stack.pop().unwrap_or(ASTNode::ConstantString("".to_owned()));
                        let parsed_arg = parse_token(args.clone()).unwrap_or(ASTNode::ConstantString(args));
                        let node = ASTNode::Function(Functions::Push, vec![ Box::new(me) ,Box::new(parsed_arg)]);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    },
                    Functions::Pop => {
                        // Pop takes no arguments and the preceeding token
                        let me = stack.pop().unwrap_or(ASTNode::ConstantString("".to_owned()));
                        let node = ASTNode::Function(Functions::Pop, vec![ Box::new(me)]);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    },
                    Functions::Keccak256 => {
                        // Keccak256 takes arbitrary arguments 
                        // The first argument indicates the length of arguments supplied
                        let number_of_args = &tokens[id+1].clone().parse::<u64>().unwrap_or(0);
                        skip_next += 1;

                        // Get the arguments
                        let mut args = vec![];
                        for i in 0..*number_of_args as usize {
                            args.push(tokens[id+i+2].clone());
                        }
                        skip_next += *number_of_args;
                        let ast_args = args.iter().map(|x| parse_token(x.clone()).unwrap()).collect::<Vec<ASTNode>>();
                        let mut boxed_args = ast_args.iter().map(|x| Box::new(x.clone())).collect::<Vec<Box<ASTNode>>>();
                        let node = ASTNode::Function(Functions::Keccak256, boxed_args);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    },
                }
            }else{

                // Parse array
                if token.starts_with('['){
                    is_array = true;
                    continue;
                }

                if token.ends_with(']'){
                    is_array = false;
                    // Push Array on stack
                    stack.push(ASTNode::Array(arr.clone()));
                    ast_vec.push(ASTNode::Array(arr.clone()));
                    arr.clear();
                    continue;
                }

                if is_array {
                    // Append to array
                    let parsed_token = parse_token(token.clone()).unwrap();
                    arr.push(Box::new(parsed_token));
                    continue;
                }

                // Parse normal token
                let node = match parse_token(token.clone()){
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
                            if token.starts_with('$'){
                                Ok(ASTNode::Variable(token[1..].to_string(), get_variable_map_instance()))
                            }
                            else {
                                Ok(ASTNode::ConstantString(token))
                            }
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

// #[derive(Debug, Clone)]
// pub enum Token{
//     Word(String),
//     Function(String),
// }

// impl Token {
//     pub fn is_empty(&self) -> bool{
//         match self {
//             Token::Word(w) => {
//                 w.is_empty()
//             },
//             Token::Function(f) => {
//                 f.is_empty()
//             }
//         }
//     }
// }

// pub fn tokenize(text: String) -> Vec<Token> {
//     let mut tokens: Vec<Token> = vec![];

//     let mut current_token = String::new();
//     let mut state = false; // false is outside function true is inside function

//     for c in text.chars(){
//         match c {
//             ' ' => {
//                 if !current_token.is_empty() {
//                     tokens.push(Token::Word(current_token.clone()));
//                     current_token.clear();
//                 }
//             },
//             '(' => {
//                 if let Ok(_) = Functions::from_str(&current_token.clone()[1..].to_string()){
//                     println!("{} is a function", current_token.clone());
//                     state = true;
//                     tokens.push(Token::Function(current_token.clone()));
//                 }else{
//                     tokens.push(Token::Word(current_token.clone()));
//                 }
//                 current_token.clear();
//                 current_token.push(c);
//                 tokens.push(Token::Word(current_token.clone()));
//                 current_token.clear();
//             },
//             ')' => {
//                 if state {
//                     state = false;
//                 }
//                 tokens.push(Token::Word(current_token.clone()));
//                 current_token.push(c);
//                 tokens.push(Token::Word(current_token.clone()));
//                 current_token.clear();
//             },
//             _ => {
//                 current_token.push(c);
//             }
//         }
//     }

//     if !current_token.is_empty() {
//         tokens.push(Token::Word(current_token.clone()));
//     }

//         // Remove empty tokens
//         tokens.retain(|x| !x.is_empty());

//     tokens
// }

pub fn tokenize(text: String) -> Vec<String> {
    let mut tokens: Vec<String> = vec![];

    let mut current_token = String::new();
    let mut state = false; // false is outside function true is inside function

    let mut is_array = false;
    for c in text.chars(){
        match c {
            ' ' | ',' | '.' => {
                if !current_token.is_empty() && !is_array {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            },
            '(' => {
                if state == false {
                    if let Ok(_) = Functions::from_str(&current_token.clone()){
                        state = true;
                        tokens.push(current_token.clone());
                    }else{
                        tokens.push(current_token.clone());
                    }
                    current_token.clear();
                }
                current_token.push(c);
                tokens.push(current_token.clone());
                current_token.clear();
            },
            ')' => {
                if state {
                    state = false;
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                current_token.push(c);
                tokens.push(current_token.clone());
                current_token.clear();
            },
            _ => {
                current_token.push(c);
            }
        }
    }

    if !current_token.is_empty() {
        tokens.push(current_token.clone());
    }

    tokens.retain(|x| !x.is_empty());

    tokens
}

/// Build an AST and return the root node
pub fn build_ast_root(text: &str) -> Result<ASTNode, &'static str>{
    let tokens = tokenize(text.to_string());
    if let Ok(postfix) = shunting_yard_algorithm(tokens){
        if let Ok((_, root)) = parse_postfix(postfix){
            Ok(root)
        }else{
            Err("Invalid AST")
        }
    }else{
        Err("Invalid AST")
    }
}

pub fn build_ast(text: &str) -> Result<(Vec<ASTNode>, ASTNode), &'static str>{
    let tokens = tokenize(text.to_string());
    if let Ok(postfix) = shunting_yard_algorithm(tokens){
        if let Ok((ast, root)) = parse_postfix(postfix){
            Ok((ast, root))
        }else{
            Err("Invalid AST")
        }
    }else{
        Err("Invalid AST")
    }
}

pub fn encode_packed(tokens: &Vec<ASTConstant>) -> Option<Vec<u8>> {
    let mut max = 0;
    for token in tokens {
        max += max_encoded_length(token);
    }
    
    // Encode
    let mut b = Vec::with_capacity(max);
    for token in tokens {
        encode_token(token, &mut b, false);
    }
    Some(b)
}

fn encode_token(token: &ASTConstant, out: &mut Vec<u8>, in_array: bool) {
    match token {
        ASTConstant::Number(n) => {
            let buf  = n.to_be_bytes();
            let start = if in_array { 0 } else { 32 - u256::leading_zeros(n.to_be())} as usize;
            out.extend_from_slice(&buf[start..32]);
        },
        ASTConstant::SignedNumber(n) => {
            let buf  = n.to_be_bytes();
            let start = if in_array { 0 } else { 32 - i256::leading_zeros(n.to_be())} as usize;
            out.extend_from_slice(&buf[start..32]);
        },
        ASTConstant::Bool(b) => {
            if in_array { 
                out.extend_from_slice(&[0;31])
            }
            out.push(*b as u8);
        },
        ASTConstant::String(s) => {
            if s.starts_with("0x") && s.len() == 42 { // Address
                if in_array {
                    out.extend_from_slice(&[0;12]);
                }
                out.extend_from_slice(&s.as_bytes());
            }else{
                out.extend_from_slice(s.as_bytes());
            }
        },
        ASTConstant::Array(vec) => {
            for t in vec {
                encode_token(t, out, true);
            }
        }
    }
}

fn max_encoded_length(t: &ASTConstant) -> usize {
    match t {
        ASTConstant::Number(_) | ASTConstant::SignedNumber(_) => {
            32
        },
        ASTConstant::String(s) => {
            if s.starts_with("0x") && s.len() == 42 { // Address
                20
            }else{
                s.len()
            }
        },
        ASTConstant::Bool(b) => {
            1
        },
        ASTConstant::Array(vec) => vec.iter().map(|x| max_encoded_length(x).max(32)).sum()
    }
}

// /// This macro builds an AST from a string
// /// The input is a string with the infix notation separated by spaces
// /// The return type is a tuple with the first entry being the full AST and the second entry being the root node
// #[macro_export]
// macro_rules! build_ast {
//     ($str_pattern:expr) => {
//         match parse_postfix(shunting_yard_algorithm(tokenize($str_pattern.to_string())).unwrap()){
//             Ok((ast, root)) => {
//                 (ast, root)
//             },
//             Err(e) => {
//                 panic!("{}", e);
//             }
//         }
//     };
// }

#[cfg(test)]
mod test_ast{
    use crate::properties::ast::*;
    
    #[test]
    fn test_tokenizer() {
        // let text = "$event_data.slice(0, 64) > 0 && $event_data.slice(0, 64) < 100".to_string();
        let text = "keccak256(10, 14,3, 17) * ( 15 + 5 ).as(hex) - 0xff".to_string();
        let tokens = tokenize(text.clone());
        println!("{:?}", tokens);

        let test = text.split(" ").map(|s| s.to_string()).collect::<Vec<String>>();
        println!("{:?}", test);
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
        let root = build_ast_root("5 == 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("{}: {}", const_type, value);
    }

    #[test]
    fn test_shunting_yard(){
        // 5 + 5 > 17 - (5 - neg 10)
        let tokens = vec!["keccak256", "(", "10", "14", "3", "hello", ")", "*", "(", "15", "+", "5", ")", "as", "(", "hex", ")", "-", "0xff"];
        let tokens_str: Vec<String> = tokens.iter().map(|x| x.to_string()).collect();
        //let tokens: Vec<String> = vec!["5".to_owned(), ">".to_owned(), "(".to_owned(), "6".to_owned(), "+".to_owned(), "5".to_owned(), ")".to_owned()];

        let output = shunting_yard_algorithm(tokens_str);
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
        let root = build_ast_root("5 > 4").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Greater: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // Less
        let root = build_ast_root("3 < 4").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Less: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // GreaterOrEqual
        let root = build_ast_root("5 >= 4").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("GreaterOrEqual: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // LessOrEqual
        let root = build_ast_root("4 <= 4").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("LessOrEqual: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // Equal
        let root = build_ast_root("5 == 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Equal: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // NotEqual
        let root = build_ast_root("5 != 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("NotEqual: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "false");

        // Not
        let root = build_ast_root("! true").unwrap();
        
        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Not: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "false");

        // Add
        let root = build_ast_root("5 + 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Add: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "10");

        // Subtract
        let root = build_ast_root("5 - 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Subtract: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "0");

        // Multiply
        let root = build_ast_root("5 * 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Multiply: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "25");

        // Divide
        let root = build_ast_root("5 / 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Divide: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "1");

        // Modulo
        let root = build_ast_root("5 % 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Modulo: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "0");

        // Negate
        let root = build_ast_root("neg 5").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("Negate: {}: {}", const_type, value);
        assert_eq!(const_type, "SignedNumber");
        assert_eq!(value, "-5");

    }

    #[test]
    fn test_complex_ast(){
        // Test combination of Arithmetic and Logic Operators
        let root = build_ast_root("5 + 5 > 7").unwrap();


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
        let root = build_ast_root("( 17 * 3 ) % 10 == 1").unwrap();

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

        let root = build_ast_root("$x == 5").unwrap();

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

        let root = build_ast_root("$x == milestone").unwrap();
        
        
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

        let root = build_ast_root("$arr.at(1) != 1").unwrap();

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

        let root = build_ast_root("$arr == $arr2").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "true");

        let root = build_ast_root("$arr != $arr3").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "true");

        println!("{:?}", get_var!("arr4").unwrap());

    }

    #[test]
    fn convert_values(){
        set_var!("a", "0xff");

        let root = build_ast_root("$a == 255").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "true");

        let root = build_ast_root("255 == $a").unwrap();
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

        let root = build_ast_root("neg $arr").unwrap();

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

        let root = build_ast_root("$arr + 1").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "[1,2,3,4]");
        assert_eq!(get_var!(value "arr").unwrap(), "[0,1,2,3]");

        let root = build_ast_root("$arr + 5 > 4").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "true");

        let root = build_ast_root("neg $arr.at(1) == 1").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "false");

        let root = build_ast_root("[0,1,2,3,4].contains(2)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "true");
    }

    #[test]
    fn test_contains() {
        set_var!("arr", "[0,1,2,3]");

        let root = build_ast_root("$arr.contains(1)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "true");

        let root = build_ast_root("$arr.contains(60)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "false");
    }

    #[test]
    fn test_conversion() {
        set_var!("a", "255");
        let root = build_ast_root("$a.as(hex)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "0xff");
    }

    #[test]
    fn test_string_arithmetic(){
        let root = build_ast_root("0xff.as(u256) - 1").unwrap();

        let val = root.evaluate().unwrap();
        let(const_type, value) = val.get_constant_info();
        println!("{}: {}", const_type, value);
    }

    #[test]
    fn test_variable_conversion(){
        set_var!("a", "255");
        let root = build_ast_root("( $a - 1 ).as(hex)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "0xfe");

    }

    #[test]
    fn test_slices() {
        set_var!("arr", "[0,1,2,3]");

        let root = build_ast_root("$arr.slice(1,3)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "[1,2]");

        set_var!("string", "hello");
        let root = build_ast_root("$string.slice(1,5)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "ello");
    }

    #[test]
    fn test_push() {

        let root = build_ast_root("hello.push(a)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "helloa");

        set_var!("some_array", "[0,1,2,3]");

        let root = build_ast_root("$some_array.push(5)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        println!("{:?}", get_var!("some_array").unwrap());
        assert_eq!(ret, "true");

        set_var!("num", 10);
        let root = build_ast_root("$some_array.push($num)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        println!("{:?}", get_var!("some_array").unwrap());
        assert_eq!(ret, "true");


    }

    #[test]
    fn test_pop() {
        set_var!("some_array", "[0,1,2,3]");
        let root = build_ast_root("$some_array.pop()").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        println!("{:?}", get_var!("some_array").unwrap());
    }

    #[test]
    fn test_as_hex_again() {
        let root = build_ast_root("e998908042a5043d06846c76bced8fdc5f4e5e91.as(hex)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "0xe998908042a5043d06846c76bced8fdc5f4e5e91");

    }

    #[test]
    fn test_keccak256(){
        let root = build_ast_root("keccak256(2, 0xe5752128B13c709d2A7E5348E601a016136a3F28, 1000000000000000000)").unwrap();

        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "0xda907f151946daa4671efcfbd42543ab19dd868bd4f6ab3e7d330746c7683c39");

    }

    #[test]
    fn test_print(){
        let root = build_ast_root("5 + 5 == ( 15 - 5 )").unwrap();
        root.print("");
        let root = build_ast_root("( $var.push(15) && true ) || 16 / 4 > 3").unwrap();
        root.print("");
        let root = build_ast_root("[ 0, 1, 2, 3, 4, hello ]").unwrap();
        root.print("");
    }
}