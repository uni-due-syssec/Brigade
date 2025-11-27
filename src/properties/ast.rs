use core::panic;
use std::collections::{ HashMap, VecDeque };
use std::env::args;
use std::fs;
use std::mem::uninitialized;
use std::path::Path;

use crate::utils::Evaluation;
use crate::{ get_var, set_var, utils };

use super::error::ASTError;

use super::environment::{ get_variable, get_variable_map_instance, VarValues, VariableMap };
use ethnum::{ i256, u256 };
use owo_colors::OwoColorize;
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use sha3::Digest;
use std::str::FromStr;

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
    Address,
    Unknown(String),
}

impl From<&str> for ConversionTarget {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "string" | "'string'" => ConversionTarget::String,
            "u256" | "'u256'" => ConversionTarget::Number,
            "i256" | "'i256'" => ConversionTarget::SignedNumber,
            "hex" | "'hex'" => ConversionTarget::Hex,
            "address" | "'address'" => ConversionTarget::Address,
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

impl ArithmeticOperator {
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
pub enum Functions {
    Contains, // Returns true if ASTNode Array contains a ASTNode value contains(value)
    At, // Returns the ASTNode value at index at(index)
    As, // Converts the value to correct representation as(type)
    Slice, // Slice Strings and return a String with character from to slice(start inclusive, end exclusive)
    Push, // Push value to the end of the keystore
    Pop, // Pop value from the end of the keystore
    Keccak256, // Keccak256 Hash
    Insert, // Insert into Map
    Remove, // Remove key from Map
    Get, // Get Value by Key from Map
    Assign, // Set Variable
    ToLower, // Transform String into lower case
    ToUpper, // Transform String into upper case
    Custom, // RPC Calls into a Blockchain
    Require, // Require a condition to execute a statement require(cond, stmt)
}

impl Functions {
    pub fn to_string(&self) -> &str {
        match self {
            Functions::Contains => "contains",
            Functions::At => "at",
            Functions::As => "as",
            Functions::Slice => "slice",
            Functions::Push => "push",
            Functions::Pop => "pop",
            Functions::Keccak256 => "keccak256",
            Functions::Insert => "insert",
            Functions::Remove => "remove",
            Functions::Get => "get",
            Functions::Assign => "assign",
            Functions::ToLower => "tolower",
            Functions::ToUpper => "toupper",
            Functions::Custom => "call",
            Functions::Require => "require",
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
            "insert" => Ok(Functions::Insert),
            "remove" => Ok(Functions::Remove),
            "get" => Ok(Functions::Get),
            "assign" => Ok(Functions::Assign),
            "tolower" | "toLower" => Ok(Functions::ToLower),
            "toupper" | "toUpper" => Ok(Functions::ToUpper),
            "call" => Ok(Functions::Custom),
            "require" => Ok(Functions::Require),
            _ => Err(ASTError::InvalidFunction(string.to_owned())),
        }
    }

    pub fn get_args(string: &str) -> Option<Vec<String>> {
        let s = string[0..string.len() - 1].to_owned();
        Some(
            s
                .split(",")
                .map(|s| s.trim().to_string())
                .collect()
        )
    }
}

#[test]
fn test_args() {
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
    Map(HashMap<String, Box<ASTNode>>),

    // Variable
    Variable(String), // String points to a variable on the VariableMap

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
        if s.starts_with('$') {
            return ASTNode::Variable(s[1..].to_owned());
        }
        ASTNode::ConstantString(s)
    }
}

impl From<&str> for ASTNode {
    fn from(s: &str) -> Self {
        if s.starts_with("$") {
            return ASTNode::Variable(s[1..].to_owned());
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
    Map(HashMap<String, ASTConstant>),
}

impl ASTConstant {
    pub fn get_map(&self) -> &HashMap<String, ASTConstant> {
        match self {
            ASTConstant::Map(v) => v,
            _ => panic!(),
        }
    }

    pub fn convert(&self, target: ConversionTarget) -> Result<ASTConstant, ASTError> {
        match target {
            ConversionTarget::String => Ok(ASTConstant::String(self.get_value())),
            ConversionTarget::Number => {
                match self {
                    ASTConstant::Number(v) => Ok(ASTConstant::Number(*v)),
                    ASTConstant::SignedNumber(v) => Ok(ASTConstant::SignedNumber(*v)),
                    ASTConstant::String(v) => {
                        if v == "0x" {
                            return Err(ASTError::InvalidNumberConversion(v.clone()));
                        }
                        if v.starts_with("0x") {
                            Ok(ASTConstant::Number(u256::from_str_hex(v).unwrap()))
                        } else if v.starts_with("u256:") || v.starts_with("i256:") {
                            Ok(ASTConstant::Number(u256::from_str(&v[5..]).unwrap()))
                        } else {
                            let num = u256::from_str(&v);
                            match num {
                                Ok(v) => {
                                    // println!("v: {}", v);
                                    Ok(ASTConstant::Number(v))
                                }
                                Err(_) => {
                                    let hex_num = u256::from_str_radix(&v, 16);
                                    match hex_num {
                                        Ok(v) => {
                                            // println!("v: {}", v);
                                            Ok(ASTConstant::Number(v))
                                        }
                                        Err(e) =>
                                            Err(
                                                ASTError::InvalidConversion(
                                                    v.to_string(),
                                                    "number".to_string()
                                                )
                                            ),
                                    }
                                }
                            }
                        }
                    }
                    _ =>
                        Err(
                            ASTError::InvalidConversion(
                                self.get_value().to_string(),
                                "number".to_string()
                            )
                        ),
                }
            }
            ConversionTarget::SignedNumber =>
                match self {
                    ASTConstant::Number(v) => Ok(ASTConstant::SignedNumber(v.as_i256())),
                    ASTConstant::SignedNumber(v) => Ok(ASTConstant::SignedNumber(*v)),
                    ASTConstant::String(v) => {
                        if v.starts_with("0x") {
                            Ok(ASTConstant::SignedNumber(i256::from_str_hex(v).unwrap()))
                        } else if v.starts_with("u256:") || v.starts_with("i256:") {
                            Ok(ASTConstant::SignedNumber(i256::from_str(&v[5..]).unwrap()))
                        } else {
                            let num = v.parse::<i256>();
                            match num {
                                Ok(v) => Ok(ASTConstant::SignedNumber(v)),
                                Err(e) =>
                                    Err(
                                        ASTError::InvalidConversion(
                                            v.to_string(),
                                            "signed number".to_string()
                                        )
                                    ),
                            }
                        }
                    }
                    _ =>
                        Err(
                            ASTError::InvalidConversion(
                                self.get_value().to_string(),
                                "signed number".to_string()
                            )
                        ),
                }
            ConversionTarget::Hex => {
                match self {
                    ASTConstant::Number(v) => Ok(ASTConstant::String(format!("0x{:x}", *v))),
                    ASTConstant::SignedNumber(v) => Ok(ASTConstant::String(format!("0x{:x}", *v))),
                    ASTConstant::String(v) => {
                        if v.starts_with("0x") {
                            Ok(ASTConstant::String(v.to_string()))
                        } else {
                            // Add the prefix to already correct hex_strings. Check if correct hex number
                            match u256::from_str_radix(v, 16) {
                                Ok(v) => Ok(ASTConstant::String(format!("0x{:x}", v))),
                                Err(e) => {
                                    // Check if other encoding:
                                    match bs58::decode(v).into_vec() {
                                        Ok(v) => {
                                            Ok(ASTConstant::String(format!("0x{}", hex::encode(v))))
                                        }
                                        Err(e) =>
                                            Err(
                                                ASTError::InvalidConversion(
                                                    v.to_string(),
                                                    "hex".to_string()
                                                )
                                            ),
                                    }
                                }
                            }
                        }
                    }
                    _ =>
                        Err(
                            ASTError::InvalidConversion(
                                self.get_value().to_string(),
                                "hex".to_string()
                            )
                        ),
                }
            }
            ConversionTarget::Address =>
                match self {
                    ASTConstant::String(v) => {
                        let mut resulting_address = String::new();
                        let mut unprefixed_addr = v.as_str();
                        if v.starts_with("0x") {
                            unprefixed_addr = &v[2..];
                        }
                        unprefixed_addr = unprefixed_addr.trim_start_matches('0');

                        let missing_zeros = 40 - unprefixed_addr.len();
                        resulting_address += "0x";
                        resulting_address.push_str(&"0".repeat(missing_zeros));
                        resulting_address.push_str(unprefixed_addr);

                        Ok(ASTConstant::String(resulting_address))
                    }
                    _ =>
                        Err(
                            ASTError::InvalidConversion(
                                self.get_value().to_string(),
                                "address".to_string()
                            )
                        ),
                }
            ConversionTarget::Unknown(s) => {
                println!("Unknown conversion target {}", s);
                Err(ASTError::UnknownConversionTarget(s))
            }
        }
    }

    pub fn get_constant_info(&self) -> (&str, String) {
        match self {
            ASTConstant::Bool(value) => ("Bool", value.to_string()),
            ASTConstant::Number(value) => ("Number", value.to_string()),
            ASTConstant::SignedNumber(value) => ("SignedNumber", value.to_string()),
            ASTConstant::String(value) => ("String", value.clone()),
            ASTConstant::Array(value) => ("Array", format!("{:?}", value)),
            ASTConstant::Map(value) => ("Map", format!("{:?}", value)),
        }
    }

    pub fn get_value(&self) -> String {
        match self {
            ASTConstant::Bool(value) => value.to_string(),
            ASTConstant::Number(value) => value.to_string(),
            ASTConstant::SignedNumber(value) => value.to_string(),
            ASTConstant::String(value) => value.clone(),
            ASTConstant::Array(value) => {
                let s = value
                    .iter()
                    .map(|value| value.get_value())
                    .collect::<Vec<String>>()
                    .join(",");
                return format!("[{}]", s);
            }
            ASTConstant::Map(value) => {
                let s = value
                    .iter()
                    .map(|(key, value)| format!("{}: {}", key, value.get_value()))
                    .collect::<Vec<String>>()
                    .join(",");
                return format!("{{{}}}", s);
            }
        }
    }

    pub fn parse(value: String) -> Self {
        if value.starts_with("[") {
            let v = value[1..value.len() - 1]
                .split(",")
                .map(|x: &str| x.to_string())
                .collect::<Vec<String>>();
            let mut arr = Vec::new();
            for s in v.iter() {
                arr.push(ASTConstant::parse(s.to_string()));
            }
            ASTConstant::Array(arr)
        } else {
            match value.parse::<u256>() {
                Ok(value) => ASTConstant::Number(value),
                Err(_) =>
                    match value.parse::<bool>() {
                        Ok(value) => ASTConstant::Bool(value),
                        Err(_) =>
                            match value.parse::<i256>() {
                                Ok(value) => ASTConstant::SignedNumber(value),
                                Err(_) => ASTConstant::String(value),
                            }
                    }
            }
        }
    }
}

impl ASTNode {
    pub fn print(&self, prefix: &str) {
        match self {
            ASTNode::ConstantBool(b) => println!("{}└── {}: {}", prefix, "Bool".yellow(), b),
            ASTNode::ConstantNumber(n) => println!("{}└── {}: {}", prefix, "Number".yellow(), n),
            ASTNode::ConstantSignedNumber(n) => {
                println!("{}└── {}: {}", prefix, "SignedNumber".yellow(), n)
            }
            ASTNode::ConstantString(s) => println!("{}└── {}: {}", prefix, "String".yellow(), s),
            ASTNode::Array(arr) => {
                println!("{}└── {}:", prefix, "Array".green());
                let last = arr.len() - 1;
                for (i, v) in arr.iter().enumerate() {
                    let new_prefix = if i == last { "   " } else { "│  " };
                    // println!("{}{}", prefix, new_prefix);
                    v.print(&format!("{}{}", prefix, new_prefix));
                }
            }
            ASTNode::Map(map) => {
                println!("{}└── {}:", prefix, "Map".green());
                let last = map.len() - 1;
                for (id, (k, v)) in map.iter().enumerate() {
                    let new_prefix = if id == last { "   " } else { "│  " };
                    // println!("{}{}", prefix, new_prefix);
                    v.print(&format!("{}{}", prefix, new_prefix));
                }
            }
            ASTNode::Variable(name) => {
                if let Some(v) = get_var!(name) {
                    match v.get_string() {
                        None => println!("{}└── {}: {}", prefix, name.blue(), v.get_value()),
                        Some(s) => {
                            if let Ok(u) = u256::from_str_hex(s) {
                                println!(
                                    "{}└── {}: {} ({})",
                                    prefix,
                                    name.blue(),
                                    u.green(),
                                    s.magenta()
                                );
                            } else {
                                println!("{}└── {}: {}", prefix, name.blue(), s);
                            }
                        }
                    }
                } else {
                    println!("{}└── {}: {}", prefix, "Variable".red(), name)
                }
            }
            ASTNode::UnaryArithmetic(operator, value) => {
                println!(
                    "{}└── {}: {}",
                    prefix,
                    "Arithmetic".fg_rgb::<156, 9, 95>(),
                    operator.to_string()
                );
                value.print(&format!("{}    ", prefix));
            }
            ASTNode::BinaryArithmetic(operator, left, right) => {
                println!(
                    "{}└── {}: {}",
                    prefix,
                    "Arithmetic".fg_rgb::<156, 9, 95>(),
                    operator.to_string()
                );
                left.print(&format!("{}│   ", prefix));
                right.print(&format!("{}    ", prefix));
            }
            ASTNode::UnaryLogic(operator, value) => {
                println!(
                    "{}└── {}: {}",
                    prefix,
                    "Logic".fg_rgb::<112, 9, 156>(),
                    operator.to_string()
                );
                value.print(&format!("{}    ", prefix));
            }
            ASTNode::BinaryLogic(operator, left, right) => {
                println!(
                    "{}└── {}: {}",
                    prefix,
                    "Logic".fg_rgb::<112, 9, 156>(),
                    operator.to_string()
                );
                left.print(&format!("{}│   ", prefix));
                right.print(&format!("{}    ", prefix));
            }
            ASTNode::Function(func, args) => {
                println!("{}└── {}: {}", prefix, "Function".cyan(), func.to_string());

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
            ASTNode::Map(map) => {
                let new_map = map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.evaluate().unwrap()))
                    .collect();
                Ok(ASTConstant::Map(new_map))
            }
            ASTNode::Variable(name) => {
                match get_var!(name) {
                    Some(value) => {
                        // println!("{:?} in Map {:p}", value, get_variable_map_instance());
                        Ok(value.to_ASTNode().evaluate()?)
                    }
                    // None => Err(ASTError::VariableNotFound { var: name.clone() }),
                    None => Ok(ASTConstant::String("NA".to_string())),
                }
            }
            ASTNode::UnaryArithmetic(operator, value) => {
                // Implementation of Unary Arithmetic Operations
                let val = value.evaluate()?;
                match operator {
                    ArithmeticOperator::Negate =>
                        match val {
                            ASTConstant::Number(value) => {
                                Ok(ASTConstant::SignedNumber(-value.as_i256()))
                            }
                            ASTConstant::SignedNumber(value) =>
                                Ok(ASTConstant::SignedNumber(-value)),
                            ASTConstant::Array(value) => {
                                let mut arr = Vec::new();
                                for v in value {
                                    if v.get_constant_info().0 == "Number" {
                                        arr.push(
                                            ASTConstant::SignedNumber(
                                                -v.get_value().parse::<i256>().unwrap()
                                            )
                                        );
                                    } else {
                                        arr.push(v.clone());
                                    }
                                }
                                Ok(ASTConstant::Array(arr))
                            }
                            _ =>
                                Err(
                                    ASTError::InvalidOperation(
                                        ArithmeticOperator::Negate.to_string().to_owned(),
                                        "bool".to_owned(),
                                        "string".to_owned()
                                    )
                                ),
                        }
                    _ => Err(ASTError::InvalidUnaryOperator),
                }
            }
            ASTNode::BinaryArithmetic(operator, left, right) => {
                // Implementation of Binary Arithmetic Operations
                let left = left.evaluate()?;
                let left_clone = left.clone();
                let right = right.evaluate()?;
                let right_clone = right.clone();
                match left {
                    ASTConstant::String(l) => {
                        // Convert string to number
                        match right {
                            ASTConstant::Number(r) => {
                                let conv = left_clone.convert(ConversionTarget::Number);
                                // let val = left_clone.convert(ConversionTarget::Number).unwrap();
                                if let Ok(val) = conv {
                                    let val_node = ASTNode::ConstantNumber(
                                        u256::from_str(val.get_value().as_str()).unwrap()
                                    );
                                    ASTNode::BinaryArithmetic(
                                        operator.clone(),
                                        Box::new(val_node),
                                        Box::new(ASTNode::ConstantNumber(r))
                                    ).evaluate()
                                } else {
                                    Err(conv.err().unwrap())
                                }
                            }
                            ASTConstant::SignedNumber(r) => {
                                let val = left_clone
                                    .convert(ConversionTarget::SignedNumber)
                                    .unwrap();
                                let val_node = ASTNode::ConstantSignedNumber(
                                    i256::from_str(val.get_value().as_str()).unwrap()
                                );
                                ASTNode::BinaryArithmetic(
                                    operator.clone(),
                                    Box::new(val_node),
                                    Box::new(ASTNode::ConstantSignedNumber(r))
                                ).evaluate()
                            }
                            ASTConstant::String(s) => {
                                let val = left_clone.convert(ConversionTarget::Number).unwrap();
                                let val_node = ASTNode::ConstantNumber(
                                    u256::from_str(val.get_value().as_str()).unwrap()
                                );
                                let r_val = right_clone.convert(ConversionTarget::Number).unwrap();
                                let r_val_node = ASTNode::ConstantNumber(
                                    u256::from_str(r_val.get_value().as_str()).unwrap()
                                );
                                ASTNode::BinaryArithmetic(
                                    operator.clone(),
                                    Box::new(val_node),
                                    Box::new(r_val_node)
                                ).evaluate()
                            }
                            _ =>
                                Err(
                                    ASTError::InvalidArithmeticOperator(
                                        operator.to_string().to_owned()
                                    )
                                ),
                        }
                    }
                    ASTConstant::SignedNumber(left) => {
                        match right {
                            ASTConstant::SignedNumber(right) =>
                                match operator {
                                    ArithmeticOperator::Add => {
                                        Ok(ASTConstant::SignedNumber(left + right))
                                    }
                                    ArithmeticOperator::Subtract => {
                                        Ok(ASTConstant::SignedNumber(left - right))
                                    }
                                    ArithmeticOperator::Multiply => {
                                        Ok(ASTConstant::SignedNumber(left * right))
                                    }
                                    ArithmeticOperator::Divide => {
                                        Ok(ASTConstant::SignedNumber(left / right))
                                    }
                                    ArithmeticOperator::Modulo => {
                                        Ok(ASTConstant::SignedNumber(left % right))
                                    }
                                    _ =>
                                        Err(
                                            ASTError::InvalidArithmeticOperator(
                                                operator.to_string().to_owned()
                                            )
                                        ),
                                }
                            ASTConstant::Array(value) => {
                                match operator {
                                    ArithmeticOperator::Add => {
                                        // Add value to all ASTConstants
                                        let arr = value
                                            .iter()
                                            .map(|value| {
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
                                            })
                                            .collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    ArithmeticOperator::Subtract => { todo!("Implement Subtract") }
                                    ArithmeticOperator::Multiply => {
                                        let arr = value
                                            .iter()
                                            .map(|value| {
                                                let mut element = value.clone();
                                                match &mut element {
                                                    ASTConstant::SignedNumber(num) => {
                                                        *num *= left;
                                                    }
                                                    _ => unreachable!(),
                                                }
                                                element
                                            })
                                            .collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    _ =>
                                        Err(
                                            ASTError::InvalidArithmeticOperator(
                                                operator.to_string().to_owned()
                                            )
                                        ),
                                }
                            }
                            ASTConstant::String(value) => {
                                // Try String Conversion
                                if value.starts_with("0x") {
                                    let v = utils::hex_string_to_u256(&value[2..]);
                                    match operator {
                                        ArithmeticOperator::Add => {
                                            Ok(ASTConstant::SignedNumber(left + v.as_i256()))
                                        }
                                        ArithmeticOperator::Subtract => {
                                            Ok(ASTConstant::SignedNumber(left - v.as_i256()))
                                        }
                                        ArithmeticOperator::Multiply => {
                                            Ok(ASTConstant::SignedNumber(left * v.as_i256()))
                                        }
                                        ArithmeticOperator::Divide => {
                                            Ok(ASTConstant::SignedNumber(left / v.as_i256()))
                                        }
                                        ArithmeticOperator::Modulo => {
                                            Ok(ASTConstant::SignedNumber(left % v.as_i256()))
                                        }
                                        _ =>
                                            Err(
                                                ASTError::InvalidArithmeticOperator(
                                                    operator.to_string().to_owned()
                                                )
                                            ),
                                    }
                                } else if value.starts_with("u256:") {
                                    let v = u256::from_str(&value[5..]).unwrap();
                                    match operator {
                                        ArithmeticOperator::Add => {
                                            Ok(ASTConstant::SignedNumber(left + v.as_i256()))
                                        }
                                        ArithmeticOperator::Subtract => {
                                            Ok(ASTConstant::SignedNumber(left - v.as_i256()))
                                        }
                                        ArithmeticOperator::Multiply => {
                                            Ok(ASTConstant::SignedNumber(left * v.as_i256()))
                                        }
                                        ArithmeticOperator::Divide => {
                                            Ok(ASTConstant::SignedNumber(left / v.as_i256()))
                                        }
                                        ArithmeticOperator::Modulo => {
                                            Ok(ASTConstant::SignedNumber(left % v.as_i256()))
                                        }
                                        _ =>
                                            Err(
                                                ASTError::InvalidArithmeticOperator(
                                                    operator.to_string().to_owned()
                                                )
                                            ),
                                    }
                                } else if value.starts_with("i256:") {
                                    let v = i256::from_str(&value[5..]).unwrap();
                                    match operator {
                                        ArithmeticOperator::Add => {
                                            Ok(ASTConstant::SignedNumber(left + v))
                                        }
                                        ArithmeticOperator::Subtract => {
                                            Ok(ASTConstant::SignedNumber(left - v))
                                        }
                                        ArithmeticOperator::Multiply => {
                                            Ok(ASTConstant::SignedNumber(left * v))
                                        }
                                        ArithmeticOperator::Divide => {
                                            Ok(ASTConstant::SignedNumber(left / v))
                                        }
                                        ArithmeticOperator::Modulo => {
                                            Ok(ASTConstant::SignedNumber(left % v))
                                        }
                                        _ =>
                                            Err(
                                                ASTError::InvalidArithmeticOperator(
                                                    operator.to_string().to_owned()
                                                )
                                            ),
                                    }
                                } else {
                                    Err(ASTError::InvalidConversion(value, "Number".to_string()))
                                }
                            }
                            _ => Err(ASTError::InvalidBinaryOperator),
                        }
                    }
                    ASTConstant::Number(left) => {
                        match right {
                            ASTConstant::Number(right) =>
                                match operator {
                                    ArithmeticOperator::Add =>
                                        match u256::checked_add(left, right) {
                                            Some(value) => Ok(ASTConstant::Number(value)),
                                            None =>
                                                Err(
                                                    ASTError::OverflowError(
                                                        format!("{} + {}", left, right)
                                                    )
                                                ),
                                        }
                                    ArithmeticOperator::Subtract => {
                                        match u256::checked_sub(left, right) {
                                            Some(value) => Ok(ASTConstant::Number(value)),
                                            None =>
                                                Err(
                                                    ASTError::OverflowError(
                                                        format!("{} - {}", left, right)
                                                    )
                                                ),
                                        }
                                    }
                                    ArithmeticOperator::Multiply => {
                                        match u256::checked_mul(left, right) {
                                            Some(value) => Ok(ASTConstant::Number(value)),
                                            None =>
                                                Err(
                                                    ASTError::OverflowError(
                                                        format!("{} * {}", left, right)
                                                    )
                                                ),
                                        }
                                    }
                                    ArithmeticOperator::Divide => {
                                        match u256::checked_div(left, right) {
                                            Some(value) => Ok(ASTConstant::Number(value)),
                                            None =>
                                                Err(
                                                    ASTError::OverflowError(
                                                        format!("{} / {}", left, right)
                                                    )
                                                ),
                                        }
                                    }
                                    ArithmeticOperator::Modulo =>
                                        Ok(ASTConstant::Number(left % right)),
                                    _ =>
                                        Err(
                                            ASTError::InvalidArithmeticOperator(
                                                operator.to_string().to_owned()
                                            )
                                        ),
                                }
                            ASTConstant::Array(value) => {
                                match operator {
                                    ArithmeticOperator::Add => {
                                        // Add value to all ASTConstants
                                        let arr = value
                                            .iter()
                                            .map(|value| {
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
                                            })
                                            .collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    ArithmeticOperator::Subtract => { todo!("Implement Subtract") }
                                    ArithmeticOperator::Multiply => {
                                        let arr = value
                                            .iter()
                                            .map(|value| {
                                                let mut element = value.clone();
                                                match &mut element {
                                                    ASTConstant::Number(num) => {
                                                        *num *= left;
                                                    }
                                                    _ => unreachable!(),
                                                }
                                                element
                                            })
                                            .collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    _ =>
                                        Err(
                                            ASTError::InvalidArithmeticOperator(
                                                operator.to_string().to_owned()
                                            )
                                        ),
                                }
                            }
                            ASTConstant::SignedNumber(value) => {
                                if value >= 0 {
                                    let v = value.as_u256();
                                    match operator {
                                        ArithmeticOperator::Add => {
                                            Ok(ASTConstant::Number(left + v))
                                        }
                                        ArithmeticOperator::Subtract => {
                                            Ok(ASTConstant::Number(left - v))
                                        }
                                        ArithmeticOperator::Multiply => {
                                            Ok(ASTConstant::Number(left * v))
                                        }
                                        ArithmeticOperator::Divide => {
                                            Ok(ASTConstant::Number(left / v))
                                        }
                                        ArithmeticOperator::Modulo => {
                                            Ok(ASTConstant::Number(left % v))
                                        }
                                        _ =>
                                            Err(
                                                ASTError::InvalidArithmeticOperator(
                                                    operator.to_string().to_owned()
                                                )
                                            ),
                                    }
                                } else {
                                    match operator {
                                        ArithmeticOperator::Add => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() + value))
                                        }
                                        ArithmeticOperator::Subtract => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() - value))
                                        }
                                        ArithmeticOperator::Multiply => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() * value))
                                        }
                                        ArithmeticOperator::Divide => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() / value))
                                        }
                                        ArithmeticOperator::Modulo => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() % value))
                                        }
                                        _ =>
                                            Err(
                                                ASTError::InvalidArithmeticOperator(
                                                    operator.to_string().to_owned()
                                                )
                                            ),
                                    }
                                }
                            }
                            ASTConstant::String(value) => {
                                // Try String Conversion
                                if value.starts_with("0x") {
                                    let v = utils::hex_string_to_u256(&value[2..]);
                                    match operator {
                                        ArithmeticOperator::Add => {
                                            Ok(ASTConstant::Number(left + v))
                                        }
                                        ArithmeticOperator::Subtract => {
                                            Ok(ASTConstant::Number(left - v))
                                        }
                                        ArithmeticOperator::Multiply => {
                                            Ok(ASTConstant::Number(left * v))
                                        }
                                        ArithmeticOperator::Divide => {
                                            Ok(ASTConstant::Number(left / v))
                                        }
                                        ArithmeticOperator::Modulo => {
                                            Ok(ASTConstant::Number(left % v))
                                        }
                                        _ =>
                                            Err(
                                                ASTError::InvalidArithmeticOperator(
                                                    operator.to_string().to_owned()
                                                )
                                            ),
                                    }
                                } else if value.starts_with("u256:") {
                                    let v = u256::from_str(&value[5..]).unwrap();
                                    match operator {
                                        ArithmeticOperator::Add => {
                                            Ok(ASTConstant::Number(left + v))
                                        }
                                        ArithmeticOperator::Subtract => {
                                            Ok(ASTConstant::Number(left - v))
                                        }
                                        ArithmeticOperator::Multiply => {
                                            Ok(ASTConstant::Number(left * v))
                                        }
                                        ArithmeticOperator::Divide => {
                                            Ok(ASTConstant::Number(left / v))
                                        }
                                        ArithmeticOperator::Modulo => {
                                            Ok(ASTConstant::Number(left % v))
                                        }
                                        _ =>
                                            Err(
                                                ASTError::InvalidArithmeticOperator(
                                                    operator.to_string().to_owned()
                                                )
                                            ),
                                    }
                                } else if value.starts_with("i256:") {
                                    let v = i256::from_str(&value[5..]).unwrap();
                                    match operator {
                                        ArithmeticOperator::Add => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() + v))
                                        }
                                        ArithmeticOperator::Subtract => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() - v))
                                        }
                                        ArithmeticOperator::Multiply => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() * v))
                                        }
                                        ArithmeticOperator::Divide => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() / v))
                                        }
                                        ArithmeticOperator::Modulo => {
                                            Ok(ASTConstant::SignedNumber(left.as_i256() % v))
                                        }
                                        _ =>
                                            Err(
                                                ASTError::InvalidArithmeticOperator(
                                                    operator.to_string().to_owned()
                                                )
                                            ),
                                    }
                                } else {
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
                                        let arr = value
                                            .iter()
                                            .map(|value| {
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
                                            })
                                            .collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    ArithmeticOperator::Subtract => {
                                        let arr = value
                                            .iter()
                                            .map(|value| {
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
                                            })
                                            .collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    ArithmeticOperator::Multiply => {
                                        let arr = value
                                            .iter()
                                            .map(|value| {
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
                                            })
                                            .collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    ArithmeticOperator::Divide => {
                                        let arr = value
                                            .iter()
                                            .map(|value| {
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
                                            })
                                            .collect();
                                        Ok(ASTConstant::Array(arr))
                                    }
                                    _ =>
                                        Err(
                                            ASTError::InvalidArithmeticOperator(
                                                operator.to_string().to_owned()
                                            )
                                        ),
                                }
                            }
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    }
                    _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                }
            }
            ASTNode::UnaryLogic(operator, value) => {
                let val = value.evaluate()?;
                match val {
                    ASTConstant::Bool(value) =>
                        match operator {
                            LogicOperator::Not => Ok(ASTConstant::Bool(!value)),
                            _ => Err(ASTError::InvalidUnaryOperator),
                        }
                    ASTConstant::Array(value) =>
                        match operator {
                            LogicOperator::Not => {
                                let arr = value
                                    .iter()
                                    .map(|value| {
                                        let mut element = value.clone();
                                        match &mut element {
                                            ASTConstant::Bool(value) => {
                                                *value = !value
                                                    .to_string()
                                                    .parse::<bool>()
                                                    .unwrap();
                                            }
                                            _ => unreachable!(),
                                        }
                                        element
                                    })
                                    .collect();
                                Ok(ASTConstant::Array(arr))
                            }
                            _ => Err(ASTError::InvalidUnaryOperator),
                        }
                    _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                }
            }
            ASTNode::BinaryLogic(operator, left, right) => {
                let left = left.evaluate()?;
                let right = right.evaluate()?;
                match left {
                    ASTConstant::SignedNumber(left) =>
                        match right {
                            ASTConstant::Number(right) =>
                                match operator {
                                    LogicOperator::Equal =>
                                        Ok(ASTConstant::Bool(left == right.as_i256())),
                                    LogicOperator::NotEqual => {
                                        Ok(ASTConstant::Bool(left != right.as_i256()))
                                    }
                                    LogicOperator::Greater =>
                                        Ok(ASTConstant::Bool(left > right.as_i256())),
                                    LogicOperator::Less =>
                                        Ok(ASTConstant::Bool(left < right.as_i256())),
                                    LogicOperator::GreaterOrEqual => {
                                        Ok(ASTConstant::Bool(left >= right.as_i256()))
                                    }
                                    LogicOperator::LessOrEqual => {
                                        Ok(ASTConstant::Bool(left <= right.as_i256()))
                                    }
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            ASTConstant::SignedNumber(right) =>
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    LogicOperator::Greater => Ok(ASTConstant::Bool(left > right)),
                                    LogicOperator::Less => Ok(ASTConstant::Bool(left < right)),
                                    LogicOperator::GreaterOrEqual =>
                                        Ok(ASTConstant::Bool(left >= right)),
                                    LogicOperator::LessOrEqual =>
                                        Ok(ASTConstant::Bool(left <= right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            ASTConstant::Array(right) => {
                                todo!("Implement Array Logic with signed numbers")
                            }
                            ASTConstant::String(right) => {
                                if right.starts_with("0x") {
                                    let v = i256::from_str_hex(&right).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(left >= v))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else if right.starts_with("u256:") {
                                    let v = u256
                                        ::from_str(&right[5..])
                                        .unwrap()
                                        .as_i256();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(left >= v))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else if right.starts_with("i256:") {
                                    let v = i256::from_str(&right[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(left >= v))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else {
                                    Err(ASTError::InvalidConversion(right, "Number".to_string()))
                                }
                            }
                            _ => Err(ASTError::InvalidBinaryOperator),
                        }
                    ASTConstant::Bool(left) =>
                        match right {
                            ASTConstant::Bool(right) =>
                                match operator {
                                    LogicOperator::And => Ok(ASTConstant::Bool(left && right)),
                                    LogicOperator::Or => Ok(ASTConstant::Bool(left || right)),
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            | ASTConstant::Number(_)
                            | ASTConstant::String(_)
                            | ASTConstant::SignedNumber(_) =>
                                match operator {
                                    LogicOperator::And => Ok(ASTConstant::Bool(left)),
                                    LogicOperator::Or => Ok(ASTConstant::Bool(left)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    ASTConstant::Number(left) =>
                        match right {
                            ASTConstant::Number(right) =>
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    LogicOperator::Greater => Ok(ASTConstant::Bool(left > right)),
                                    LogicOperator::Less => Ok(ASTConstant::Bool(left < right)),
                                    LogicOperator::GreaterOrEqual =>
                                        Ok(ASTConstant::Bool(left >= right)),
                                    LogicOperator::LessOrEqual =>
                                        Ok(ASTConstant::Bool(left <= right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            ASTConstant::SignedNumber(right) =>
                                match operator {
                                    LogicOperator::Equal =>
                                        Ok(ASTConstant::Bool(left.as_i256() == right)),
                                    LogicOperator::NotEqual => {
                                        Ok(ASTConstant::Bool(left.as_i256() != right))
                                    }
                                    LogicOperator::Greater =>
                                        Ok(ASTConstant::Bool(left.as_i256() > right)),
                                    LogicOperator::Less =>
                                        Ok(ASTConstant::Bool(left.as_i256() < right)),
                                    LogicOperator::GreaterOrEqual => {
                                        Ok(ASTConstant::Bool(left.as_i256() >= right))
                                    }
                                    LogicOperator::LessOrEqual => {
                                        Ok(ASTConstant::Bool(left.as_i256() <= right))
                                    }
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            ASTConstant::String(right) => {
                                if right.starts_with("0x") {
                                    let v = u256::from_str_hex(&right).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(left >= v))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else if right.starts_with("u256:") {
                                    let v = u256::from_str(&right[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(left == v)),
                                        LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != v)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(left > v)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(left < v)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(left >= v))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(left <= v)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else if right.starts_with("i256:") {
                                    let v = i256::from_str(&right[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => {
                                            Ok(ASTConstant::Bool(left.as_i256() == v))
                                        }
                                        LogicOperator::NotEqual => {
                                            Ok(ASTConstant::Bool(left.as_i256() != v))
                                        }
                                        LogicOperator::Greater => {
                                            Ok(ASTConstant::Bool(left.as_i256() > v))
                                        }
                                        LogicOperator::Less => {
                                            Ok(ASTConstant::Bool(left.as_i256() < v))
                                        }
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(left.as_i256() >= v))
                                        }
                                        LogicOperator::LessOrEqual => {
                                            Ok(ASTConstant::Bool(left.as_i256() <= v))
                                        }
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else {
                                    Err(ASTError::InvalidConversion(right, "Number".to_string()))
                                }
                            }
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    ASTConstant::String(left) =>
                        match right {
                            ASTConstant::String(right) =>
                                match operator {
                                    LogicOperator::Equal => Ok(ASTConstant::Bool(left == right)),
                                    LogicOperator::NotEqual => Ok(ASTConstant::Bool(left != right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            ASTConstant::Number(right) => {
                                if left.starts_with("0x") {
                                    let l = u256::from_str_hex(&left).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual =>
                                            Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(l >= right))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else if left.starts_with("u256:") {
                                    let l = u256::from_str(&left[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual =>
                                            Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(l >= right))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else if left.starts_with("i256:") {
                                    let l = i256::from_str(&left[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => {
                                            Ok(ASTConstant::Bool(l == right.as_i256()))
                                        }
                                        LogicOperator::NotEqual => {
                                            Ok(ASTConstant::Bool(l != right.as_i256()))
                                        }
                                        LogicOperator::Greater => {
                                            Ok(ASTConstant::Bool(l > right.as_i256()))
                                        }
                                        LogicOperator::Less => {
                                            Ok(ASTConstant::Bool(l < right.as_i256()))
                                        }
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(l >= right.as_i256()))
                                        }
                                        LogicOperator::LessOrEqual => {
                                            Ok(ASTConstant::Bool(l <= right.as_i256()))
                                        }
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else {
                                    Err(ASTError::InvalidConversion(left, "Number".to_string()))
                                }
                            }
                            ASTConstant::SignedNumber(right) => {
                                if left.starts_with("0x") {
                                    let l = i256::from_str_hex(&left).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual =>
                                            Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(l >= right))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else if left.starts_with("u256:") {
                                    let l = i256::from_str(&left[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual =>
                                            Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(l >= right))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else if left.starts_with("i256:") {
                                    let l = i256::from_str(&left[5..]).unwrap();
                                    match operator {
                                        LogicOperator::Equal => Ok(ASTConstant::Bool(l == right)),
                                        LogicOperator::NotEqual =>
                                            Ok(ASTConstant::Bool(l != right)),
                                        LogicOperator::Greater => Ok(ASTConstant::Bool(l > right)),
                                        LogicOperator::Less => Ok(ASTConstant::Bool(l < right)),
                                        LogicOperator::GreaterOrEqual => {
                                            Ok(ASTConstant::Bool(l >= right))
                                        }
                                        LogicOperator::LessOrEqual =>
                                            Ok(ASTConstant::Bool(l <= right)),
                                        _ => Err(ASTError::InvalidBinaryOperator),
                                    }
                                } else {
                                    Err(ASTError::InvalidConversion(left, "Number".to_string()))
                                }
                            }
                            ASTConstant::Bool(right) =>
                                match operator {
                                    LogicOperator::And => Ok(ASTConstant::Bool(right)),
                                    LogicOperator::Or => Ok(ASTConstant::Bool(right)),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    ASTConstant::Array(left) => {
                        match right {
                            ASTConstant::SignedNumber(right) =>
                                match operator {
                                    LogicOperator::Greater =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::SignedNumber(num) =>
                                                            *num > right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::Less =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::SignedNumber(num) =>
                                                            *num < right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::GreaterOrEqual =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::SignedNumber(num) =>
                                                            *num >= right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::LessOrEqual =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::SignedNumber(num) =>
                                                            *num <= right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::Equal =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::SignedNumber(num) =>
                                                            *num == right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::NotEqual =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::SignedNumber(num) =>
                                                            *num != right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            ASTConstant::Number(right) =>
                                match operator {
                                    LogicOperator::Greater =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::Number(num) => *num > right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::Less =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::Number(num) => *num < right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::GreaterOrEqual =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::Number(num) => *num >= right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::LessOrEqual =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::Number(num) => *num <= right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::Equal =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::Number(num) => *num == right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    LogicOperator::NotEqual =>
                                        Ok(
                                            ASTConstant::Bool(
                                                left.iter().all(|element| {
                                                    match element {
                                                        ASTConstant::Number(num) => *num != right,
                                                        _ => unreachable!(),
                                                    }
                                                })
                                            )
                                        ),
                                    _ => Err(ASTError::InvalidBinaryOperator),
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
                                    }
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
                                    }
                                    | LogicOperator::Greater
                                    | LogicOperator::Less
                                    | LogicOperator::GreaterOrEqual
                                    | LogicOperator::LessOrEqual => {
                                        todo!("Implement logic operator for array");
                                    }
                                    _ => Err(ASTError::InvalidBinaryOperator),
                                }
                            }
                            _ => Err(ASTError::InvalidConstant(operator.to_string().to_owned())),
                        }
                    }
                    ASTConstant::Map(_) => Err(ASTError::InvalidBinaryOperator),
                }
            }
            ASTNode::Function(function_name, args) => {
                match function_name {
                    Functions::Contains => {
                        let set = args[0].evaluate()?;
                        let value = args[1].evaluate()?;
                        match set {
                            ASTConstant::Array(arr) => {
                                // println!("{}\n{}", arr.len(), value.get_value());
                                for element in arr {
                                    // println!("{}", element.get_value());
                                    if element.get_value() == value.get_value() {
                                        return Ok(ASTConstant::Bool(true));
                                    }
                                }
                                Ok(ASTConstant::Bool(false))
                                //Ok(ASTConstant::Bool(arr.iter().any(|element| element.get_value() == value.get_value())))
                            }
                            ASTConstant::String(s) => {
                                Ok(ASTConstant::Bool(s.contains(&value.get_value())))
                            }
                            _ => Err(ASTError::InvalidFunctionInvocation("contains".to_owned())),
                        }
                    }
                    Functions::At => {
                        let set = args[0].evaluate()?;
                        let index = args[1].evaluate()?;

                        let idx = index
                            .get_value()
                            .parse::<usize>()
                            .expect("index must be an integer");

                        match set {
                            ASTConstant::Array(arr) => {
                                let entry = &arr[idx];
                                match entry {
                                    ASTConstant::Bool(value) => Ok(ASTConstant::Bool(*value)),
                                    ASTConstant::Number(value) => Ok(ASTConstant::Number(*value)),
                                    ASTConstant::SignedNumber(value) => {
                                        Ok(ASTConstant::SignedNumber(*value))
                                    }
                                    ASTConstant::String(value) => {
                                        Ok(ASTConstant::String(value.to_string()))
                                    }
                                    ASTConstant::Array(value) => {
                                        Ok(ASTConstant::Array(value.clone()))
                                    }
                                    ASTConstant::Map(m) => Ok(ASTConstant::Map(m.clone())),
                                }
                            }
                            _ => Err(ASTError::InvalidFunctionInvocation("at".to_owned())),
                        }
                    }
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
                    Functions::Slice => {
                        let me = args[0].evaluate()?;
                        let start = args[1].evaluate()?;
                        let end = args[2].evaluate()?;

                        let start_index = start
                            .convert(ConversionTarget::Number)
                            .unwrap()
                            .get_value()
                            .parse::<usize>()
                            .expect("start index must be an integer");
                        let end_index = end
                            .convert(ConversionTarget::Number)
                            .unwrap()
                            .get_value()
                            .parse::<usize>()
                            .expect("end index must be an integer");

                        match me {
                            ASTConstant::Array(arr) => {
                                Ok(ASTConstant::Array(arr[start_index..end_index].to_vec()))
                            }
                            ASTConstant::String(s) => {
                                if end_index > s.len() {
                                    return Err(
                                        ASTError::InvalidSlice(
                                            s.clone(),
                                            start_index,
                                            end_index,
                                            s.len()
                                        )
                                    );
                                }
                                Ok(ASTConstant::String(s[start_index..end_index].to_string()))
                            }
                            _ => Err(ASTError::InvalidFunctionInvocation("slice".to_owned())),
                        }
                    }
                    Functions::Push => {
                        let node = args[0].clone();
                        let me = args[0].evaluate()?;
                        let value = args[1].evaluate()?;
                        match me {
                            ASTConstant::Array(arr) => {
                                if let ASTNode::Variable(name) = *node {
                                    if let Some(a) = get_var!(&name) {
                                        match a {
                                            VarValues::Array(mut inner) =>
                                                match value {
                                                    ASTConstant::Array(arr) => {
                                                        for item in arr {
                                                            inner.push(VarValues::from(item));
                                                        }
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    }
                                                    ASTConstant::Bool(v) => {
                                                        inner.push(VarValues::from(v));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    }
                                                    ASTConstant::Number(v) => {
                                                        inner.push(VarValues::from(v));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    }
                                                    ASTConstant::SignedNumber(v) => {
                                                        inner.push(VarValues::from(v));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    }
                                                    ASTConstant::String(v) => {
                                                        inner.push(VarValues::from(v));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    }
                                                    ASTConstant::Map(map) => {
                                                        inner.push(VarValues::from(map));
                                                        set_var!(name, VarValues::Array(inner));
                                                        Ok(ASTConstant::Bool(true))
                                                    }
                                                }
                                            _ => {
                                                return Err(
                                                    ASTError::InvalidFunctionInvocation(
                                                        "push".to_owned()
                                                    )
                                                );
                                            }
                                        }
                                    } else {
                                        println!("Variable not found: {}", name);
                                        // Build new Array and push
                                        match value {
                                            ASTConstant::Bool(v) => {
                                                let new_arr: Vec<VarValues> = vec![
                                                    VarValues::from(v)
                                                ];
                                                set_var!(name, new_arr);
                                                return Ok(ASTConstant::Bool(true));
                                            }
                                            ASTConstant::Number(v) => {
                                                let new_arr: Vec<VarValues> = vec![
                                                    VarValues::from(v)
                                                ];
                                                set_var!(name, new_arr);
                                                return Ok(ASTConstant::Bool(true));
                                            }
                                            ASTConstant::SignedNumber(v) => {
                                                let new_arr: Vec<VarValues> = vec![
                                                    VarValues::from(v)
                                                ];
                                                set_var!(name, new_arr);
                                                return Ok(ASTConstant::Bool(true));
                                            }
                                            ASTConstant::String(v) => {
                                                let new_arr: Vec<VarValues> = vec![
                                                    VarValues::from(v)
                                                ];
                                                set_var!(name, new_arr);
                                                return Ok(ASTConstant::Bool(true));
                                            }
                                            _ => {
                                                return Err(
                                                    ASTError::InvalidFunctionInvocation(
                                                        "push".to_owned()
                                                    )
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    Err(ASTError::InvalidFunctionInvocation("push".to_owned()))
                                }
                            }
                            ASTConstant::String(s) => {
                                let new_string = format!("{}{}", s, value.get_value());
                                if let ASTNode::Variable(name) = *node {
                                    set_var!(name, new_string.clone());
                                }
                                Ok(ASTConstant::String(new_string))
                            }
                            ASTConstant::Number(n) => {
                                let new_number = format!("{}{}", n, value.get_value());
                                Ok(ASTConstant::String(new_number))
                            }
                            ASTConstant::SignedNumber(n) => {
                                let new_number = format!("{}{}", n, value.get_value());
                                Ok(ASTConstant::String(new_number))
                            }
                            _ => {
                                return Err(ASTError::InvalidFunctionInvocation("push".to_owned()));
                            }
                        }
                    }
                    Functions::Pop => {
                        let me = args[0].clone().evaluate()?;
                        match me {
                            ASTConstant::Array(mut arr) => {
                                let last = arr.pop().unwrap();
                                if let ASTNode::Variable(name) = *args[0].clone() {
                                    set_var!(name, arr);
                                }

                                match last {
                                    ASTConstant::Bool(v) => Ok(ASTConstant::Bool(v)),
                                    ASTConstant::Number(v) => Ok(ASTConstant::Number(v)),
                                    ASTConstant::SignedNumber(v) => {
                                        Ok(ASTConstant::SignedNumber(v))
                                    }
                                    ASTConstant::String(v) => Ok(ASTConstant::String(v)),
                                    _ => Err(ASTError::InvalidFunctionInvocation("pop".to_owned())),
                                }
                            }
                            _ => Err(ASTError::InvalidFunctionInvocation("pop".to_owned())),
                        }
                    }
                    Functions::Keccak256 => {
                        let evalled_args = args
                            .iter()
                            .map(|x| x.evaluate().unwrap())
                            .collect::<Vec<ASTConstant>>();

                        let serialized_values = encode_packed(&evalled_args).unwrap();

                        let concatenated_bytes = serialized_values.as_slice();

                        // let mut hasher = sha3::Keccak256::digest(concatenated_bytes).to_vec();
                        // let hex_string = hasher
                        //     .iter()
                        //     .map(|&num| format!("{:02x}", num))
                        //     .collect::<Vec<String>>()
                        //     .join("");
                        // let s = "0x".to_string() + &hex_string;
                        // // println!("Keccak256: {}", s);
                        unimplemented!("Keccak256 is not implemented yet");
                        // Ok(ASTConstant::String(s))
                    }
                    Functions::Insert => {
                        let me = args[0].clone().evaluate()?;
                        let key = args[1].evaluate()?;
                        let value = args[2].evaluate()?;
                        match me {
                            ASTConstant::Map(mut map) => {
                                map.insert(key.get_value(), value);
                                // println!("Insert: {}", key.get_value());
                                if let ASTNode::Variable(name) = *args[0].clone() {
                                    // println!("Store: {}", name);
                                    set_var!(name, map);
                                    return Ok(ASTConstant::Bool(true));
                                }
                                return Ok(ASTConstant::Bool(false));
                            }
                            _ => {
                                return Err(
                                    ASTError::InvalidFunctionInvocation("insert".to_owned())
                                );
                            }
                        }
                    }
                    Functions::Remove => {
                        let me = args[0].evaluate()?;
                        let key = args[1].evaluate()?;

                        match me.clone() {
                            ASTConstant::Map(mut map) =>
                                match map.remove(&key.get_value()) {
                                    Some(v) => {
                                        if let ASTNode::Variable(name) = *args[0].clone() {
                                            set_var!(name, map);
                                        }
                                        Ok(v)
                                    }
                                    None => Err(ASTError::UnknownKey(key.get_value().to_string())),
                                }
                            ASTConstant::Array(mut arr) => {
                                if arr.len() == 0 {
                                    return Err(ASTError::EmptyArray);
                                }

                                let mut index = 0;
                                for a in &arr {
                                    if a.get_value() == key.get_value() {
                                        break;
                                    }
                                    index += 1;
                                }

                                if index > arr.len() - 1 {
                                    return Err(
                                        ASTError::KeyNotFound(key.get_value(), me.get_value())
                                    );
                                }

                                if index == 0 && arr[0].get_value() != key.get_value() {
                                    return Err(
                                        ASTError::KeyNotFound(key.get_value(), me.get_value())
                                    );
                                }
                                let ret = arr.remove(index);
                                if let ASTNode::Variable(name) = *args[0].clone() {
                                    set_var!(name, arr);
                                }
                                return Ok(ret);
                            }
                            _ => {
                                return Err(
                                    ASTError::InvalidFunctionInvocation("remove".to_owned())
                                );
                            }
                        }
                    }
                    Functions::Get => {
                        let me = args[0].clone().evaluate()?;
                        let key = args[1].evaluate()?;
                        match me {
                            ASTConstant::Map(map) =>
                                match map.get(&key.get_value()) {
                                    Some(value) => Ok(value.clone()),
                                    None => Err(ASTError::UnknownKey(key.get_value().to_string())),
                                }
                            _ => {
                                return Err(ASTError::InvalidFunctionInvocation("get".to_owned()));
                            }
                        }
                    }
                    Functions::Assign => {
                        let key = args[0].clone().evaluate()?;
                        let value = args[1].evaluate()?;
                        match key {
                            ASTConstant::String(s) => {
                                set_var!(s, value);
                                Ok(ASTConstant::Bool(true))
                            }
                            _ => {
                                return Err(
                                    ASTError::InvalidFunctionInvocation("assign".to_owned())
                                );
                            }
                        }
                    }
                    Functions::ToLower => {
                        let me = args[0].evaluate()?;
                        match me {
                            ASTConstant::String(s) => Ok(ASTConstant::String(s.to_lowercase())),
                            _ => Err(ASTError::InvalidFunctionInvocation("tolower".to_owned())),
                        }
                    }
                    Functions::ToUpper => {
                        let me = args[0].evaluate()?;
                        match me {
                            ASTConstant::String(s) => Ok(ASTConstant::String(s.to_uppercase())),
                            _ => Err(ASTError::InvalidFunctionInvocation("toupper".to_owned())),
                        }
                    }
                    Functions::Custom => {
                        // Index 1 = Endpoint
                        let endpoint = *args[0].clone();
                        // Index 2 = Function Name
                        let function_name = *args[1].clone();
                        // Index 3 and all following = Args
                        let args = args[2..].to_vec();

                        // Find correct endpoint
                        let f = format!(
                            "functions/{}/connection.json",
                            endpoint.evaluate().unwrap().get_value()
                        );
                        let p = Path::new(&f);

                        if !p.is_file() {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!(
                                        "wrong endpoint in call(). Check if functions/{}/connection.json exists",
                                        endpoint.evaluate().unwrap().get_value()
                                    )
                                )
                            );
                        }

                        // Read contents and get "endpoint" field
                        let contents = fs
                            ::read_to_string(p)
                            .expect("Something went wrong reading the file");
                        let json: Value = serde_json::from_str(&contents).unwrap();
                        let endpoint_address = json["endpoint"].as_str().unwrap();

                        // Find correct function
                        let f = format!(
                            "functions/{}/rpc.json",
                            endpoint.evaluate().unwrap().get_value()
                            // function_name.evaluate().unwrap().get_value()
                        );
                        let p = Path::new(&f);

                        // Read contents and replace all params that start with a $ sign with the respective arguments
                        let mut contents: RPCRequest = serde_json
                            ::from_str(
                                &fs
                                    ::read_to_string(p)
                                    .expect("Something went wrong reading the file")
                            )
                            .unwrap();
                        // let mut json: Value = serde_json::from_str(&contents)
                        //     .expect("Failed to convert contents to json");
                        let mut clear_args = args
                            .iter()
                            .map(|x| x.evaluate().unwrap().get_value())
                            .collect::<Vec<String>>();
                        // replace_args_in_json(&mut json, &mut clear_args);
                        clear_args.insert(0, function_name.evaluate().unwrap().get_value());
                        let s = replace_args_in_value(&mut contents, &clear_args);
                        if s.is_err() {
                            return Err(
                                ASTError::RequestReplacementError(format!("{:?}", s.unwrap_err()))
                            );
                        }
                        // let json = replace_args_in_str(&contents, &clear_args);

                        // let json = serde_json::to_string(&json).unwrap();
                        // println!("Json: {:?}", contents);
                        // Build Client and send request
                        let client = reqwest::blocking::Client::builder().build().unwrap();
                        // print!("Endpoint: {}\n", endpoint_address);
                        let resp = client.post(endpoint_address).json(&contents).send().unwrap();
                        // let resp2 = client.post(endpoint_address).json(&json).build().unwrap();
                        // println!("Request: {:?}", resp2);
                        let body = resp.text().unwrap();
                        println!("Result: {:?}", body);
                        let result: Value = serde_json::from_str(&body.as_str()).unwrap();

                        // check if message contains an error
                        if result.get("error").is_some() {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!(
                                        "Error: {}",
                                        result
                                            .get("error")
                                            .unwrap()
                                            .get("message")
                                            .unwrap()
                                            .as_str()
                                            .unwrap()
                                    )
                                )
                            );
                        }

                        let ret = ASTNode::from(result).evaluate();

                        match ret {
                            Ok(v) => Ok(v),
                            Err(e) => Err(ASTError::ExpectedJSON),
                        }
                    }
                    Functions::Require => {
                        let cond = args[0].evaluate();
                        match cond {
                            Ok(v) => {
                                if v.get_value() == "true" {
                                    // If condition is true: execute statement
                                    let stmt = args[1].evaluate();
                                    match stmt {
                                        Ok(v) => Ok(v),
                                        Err(e) => Err(e),
                                    }
                                } else {
                                    // else return false
                                    Ok(ASTConstant::Bool(false))
                                }
                            }
                            Err(e) => Err(ASTError::RequireError(e.to_string())),
                        }
                    }
                }
            }
            ASTNode::Array(val) => {
                let mut arr = vec![];
                for v in val {
                    arr.push(v.evaluate()?);
                }
                Ok(ASTConstant::Array(arr))
            }
        }
    }

    fn format(&self) -> String {
        match self {
            ASTNode::ConstantBool(value) => value.to_string(),
            ASTNode::ConstantNumber(value) => value.to_string(),
            ASTNode::ConstantSignedNumber(value) => value.to_string(),
            ASTNode::ConstantString(value) => value.clone(),
            ASTNode::Map(map) =>
                format!(
                    "{}\n",
                    map
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v.format()))
                        .collect::<Vec<String>>()
                        .join("\n")
                ),
            ASTNode::Variable(name) => get_var!(value name.as_str()).unwrap(),
            ASTNode::UnaryArithmetic(operator, value) => {
                format!("\t{}\t\n{}", operator.to_string(), value.format())
            }
            ASTNode::BinaryArithmetic(operator, left, right) => {
                format!("\t{}\n{}\t\t{}", operator.to_string(), left.format(), right.format())
            }
            ASTNode::UnaryLogic(operator, value) => {
                format!("\t{}\t\n{}", operator.to_string(), value.format())
            }
            ASTNode::BinaryLogic(operator, left, right) => {
                format!("\t{}\n{}\t\t{}", operator.to_string(), left.format(), right.format())
            }
            ASTNode::Array(values) =>
                format!(
                    "{}\n",
                    values
                        .iter()
                        .map(|value| value.format())
                        .collect::<Vec<String>>()
                        .join("\n")
                ),
            ASTNode::Function(function_name, params) =>
                format!(
                    "{}({})",
                    function_name.to_string(),
                    params
                        .iter()
                        .map(|value| value.format())
                        .collect::<Vec<String>>()
                        .join("\n")
                ),
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
                let v = value
                    .iter()
                    .map(|x| Box::new(ASTNode::from(x.clone())))
                    .collect();
                ASTNode::Array(v)
            }
            ASTConstant::Map(map) => {
                let v = map
                    .iter()
                    .map(|(k, v)| (k.clone(), Box::new(ASTNode::from(v.clone()))))
                    .collect();
                ASTNode::Map(v)
            }
        }
    }
}

impl From<Value> for ASTNode {
    fn from(value: Value) -> Self {
        match value {
            Value::Bool(value) => ASTNode::ConstantBool(value),
            Value::Number(value) => ASTNode::ConstantNumber(u256::from(value.as_u64().unwrap())),
            Value::String(value) => ASTNode::ConstantString(value),
            Value::Array(arr) => {
                let v = arr
                    .iter()
                    .map(|x| Box::new(ASTNode::from(x.clone())))
                    .collect();
                ASTNode::Array(v)
            }
            Value::Object(map) => {
                let v = map
                    .iter()
                    .map(|(k, v)| (k.clone(), Box::new(ASTNode::from(v.clone()))))
                    .collect();
                ASTNode::Map(v)
            }
            _ => ASTNode::ConstantString("None".to_string()),
        }
    }
}

/// Build the tree from a Vec of tokens and return the AST and the root node
pub fn parse_postfix(tokens: VecDeque<String>) -> Result<(Vec<ASTNode>, ASTNode), ASTError> {
    let mut ast_vec: Vec<ASTNode> = vec![];
    let mut stack: Vec<ASTNode> = vec![];

    // Array Helper
    let mut arr: Vec<Box<ASTNode>> = vec![];
    let mut is_array = false;

    // println!("Tokens: {:?}", tokens);

    for (id, token) in tokens.iter().enumerate() {
        // stack.last().unwrap_or(&ASTNode::ConstantString("None".to_string())).print("");
        // println!("_____________________________________________________");

        // println!("Token: {}", token);

        if is_operator(token.as_str()) {
            match ArithmeticOperator::from_str(token.as_str()) {
                Ok(value) =>
                    match value {
                        ArithmeticOperator::Negate => {
                            let node = ASTNode::UnaryArithmetic(
                                ArithmeticOperator::Negate,
                                Box::new(stack.pop().unwrap())
                            );
                            ast_vec.push(node.clone());
                            stack.push(node);
                        }
                        ArithmeticOperator::Add => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(
                                ArithmeticOperator::Add,
                                Box::new(left),
                                Box::new(right)
                            );
                            ast_vec.push(node.clone());
                            stack.push(node);
                        }
                        ArithmeticOperator::Subtract => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(
                                ArithmeticOperator::Subtract,
                                Box::new(left),
                                Box::new(right)
                            );
                            ast_vec.push(node.clone());
                            stack.push(node);
                        }
                        ArithmeticOperator::Multiply => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(
                                ArithmeticOperator::Multiply,
                                Box::new(left),
                                Box::new(right)
                            );
                            ast_vec.push(node.clone());
                            stack.push(node);
                        }
                        ArithmeticOperator::Divide => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(
                                ArithmeticOperator::Divide,
                                Box::new(left),
                                Box::new(right)
                            );
                            ast_vec.push(node.clone());
                            stack.push(node);
                        }
                        ArithmeticOperator::Modulo => {
                            let right = stack.pop().unwrap();
                            let left = stack.pop().unwrap();
                            let node = ASTNode::BinaryArithmetic(
                                ArithmeticOperator::Modulo,
                                Box::new(left),
                                Box::new(right)
                            );
                            ast_vec.push(node.clone());
                            stack.push(node);
                        }
                    }
                Err(_) => {
                    //println!("{} is not an Arithmetic Operator", token);
                    match LogicOperator::from_str(token.as_str()) {
                        Ok(value) =>
                            match value {
                                LogicOperator::Not => {
                                    let node = ASTNode::UnaryLogic(
                                        LogicOperator::Not,
                                        Box::new(stack.pop().unwrap())
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                                LogicOperator::And => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(
                                        LogicOperator::And,
                                        Box::new(left),
                                        Box::new(right)
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                                LogicOperator::Or => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(
                                        LogicOperator::Or,
                                        Box::new(left),
                                        Box::new(right)
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                                LogicOperator::Equal => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(
                                        LogicOperator::Equal,
                                        Box::new(left),
                                        Box::new(right)
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                                LogicOperator::NotEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(
                                        LogicOperator::NotEqual,
                                        Box::new(left),
                                        Box::new(right)
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                                LogicOperator::Greater => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(
                                        LogicOperator::Greater,
                                        Box::new(left),
                                        Box::new(right)
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                                LogicOperator::Less => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(
                                        LogicOperator::Less,
                                        Box::new(left),
                                        Box::new(right)
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                                LogicOperator::GreaterOrEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(
                                        LogicOperator::GreaterOrEqual,
                                        Box::new(left),
                                        Box::new(right)
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                                LogicOperator::LessOrEqual => {
                                    let right = stack.pop().unwrap();
                                    let left = stack.pop().unwrap();
                                    let node = ASTNode::BinaryLogic(
                                        LogicOperator::LessOrEqual,
                                        Box::new(left),
                                        Box::new(right)
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                }
                            }
                        Err(_) => {
                            println!("{} is not a Logic Operator", token);
                        }
                    }
                }
            }
        } else {
            // Parse Operand in respective type

            if let Ok(func) = Functions::from_str(token.as_str()) {
                // Parse Functions
                match func {
                    Functions::As => {
                        // As takes two arguments
                        if let Some(arg_1) = stack.pop() {
                            if let Some(arg_0) = stack.pop() {
                                let node = ASTNode::Function(
                                    Functions::As,
                                    vec![Box::new(arg_0), Box::new(arg_1)]
                                );
                                ast_vec.push(node.clone());
                                stack.push(node);
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument.as({:?})", arg_1)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .as()")
                                )
                            );
                        }
                    }
                    Functions::Contains => {
                        // Contains takes two arguments

                        if let Some(arg_1) = stack.pop() {
                            if let Some(arg_0) = stack.pop() {
                                let node = ASTNode::Function(
                                    Functions::Contains,
                                    vec![Box::new(arg_0), Box::new(arg_1)]
                                );
                                ast_vec.push(node.clone());
                                stack.push(node);
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument.contains({:?})", arg_1)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .contains()")
                                )
                            );
                        }
                    }
                    Functions::At => {
                        // At takes two arguments
                        if let Some(arg_1) = stack.pop() {
                            if let Some(arg_0) = stack.pop() {
                                let node = ASTNode::Function(
                                    Functions::At,
                                    vec![Box::new(arg_0), Box::new(arg_1)]
                                );
                                ast_vec.push(node.clone());
                                stack.push(node);
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument.at({:?})", arg_1)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .at()")
                                )
                            );
                        }
                    }
                    Functions::Slice => {
                        // Slice takes two arguments and the preceeding token

                        if let Some(arg_2) = stack.pop() {
                            if let Some(arg_1) = stack.pop() {
                                if let Some(arg_0) = stack.pop() {
                                    let node = ASTNode::Function(
                                        Functions::Slice,
                                        vec![Box::new(arg_0), Box::new(arg_1), Box::new(arg_2)]
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                } else {
                                    return Err(
                                        ASTError::InvalidFunctionInvocation(
                                            format!(
                                                "Missing argument .slice({:?}, {:?})",
                                                arg_1,
                                                arg_2
                                            )
                                        )
                                    );
                                }
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument .slice({:?})", arg_2)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .slice()")
                                )
                            );
                        }
                    }
                    Functions::Push => {
                        // Push takes two arguments

                        if let Some(arg_1) = stack.pop() {
                            if let Some(arg_0) = stack.pop() {
                                let node = ASTNode::Function(
                                    Functions::Push,
                                    vec![Box::new(arg_0), Box::new(arg_1)]
                                );
                                ast_vec.push(node.clone());
                                stack.push(node);
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument .push({:?})", arg_1)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .push()")
                                )
                            );
                        }
                    }
                    Functions::Pop => {
                        // Pop takes one argument
                        if let Some(arg_0) = stack.pop() {
                            let node = ASTNode::Function(Functions::Pop, vec![Box::new(arg_0)]);
                            ast_vec.push(node.clone());
                            stack.push(node);
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .pop()")
                                )
                            );
                        }
                    }
                    Functions::Keccak256 => {
                        unimplemented!("Not implemented right now");
                        // Keccak256 takes arbitrary arguments
                        // The first argument indicates the length of arguments supplied
                        let number_of_args = &tokens[id + 1].clone().parse::<u64>().unwrap_or(0);

                        // Get the arguments
                        let mut args = vec![];
                        for i in 0..*number_of_args as usize {
                            args.push(tokens[id + i + 2].clone());
                        }
                        let ast_args = args
                            .iter()
                            .map(|x| parse_token(x.clone()).unwrap())
                            .collect::<Vec<ASTNode>>();
                        let mut boxed_args = ast_args
                            .iter()
                            .map(|x| Box::new(x.clone()))
                            .collect::<Vec<Box<ASTNode>>>();
                        let node = ASTNode::Function(Functions::Keccak256, boxed_args);
                        ast_vec.push(node.clone());
                        stack.push(node);
                    }
                    Functions::Insert => {
                        // Insert takes two arguments (key, value) and the target

                        if let Some(value) = stack.pop() {
                            if let Some(key) = stack.pop() {
                                if let Some(target) = stack.pop() {
                                    let node = ASTNode::Function(
                                        Functions::Insert,
                                        vec![Box::new(target), Box::new(key), Box::new(value)]
                                    );
                                    ast_vec.push(node.clone());
                                    stack.push(node);
                                } else {
                                    return Err(
                                        ASTError::InvalidFunctionInvocation(
                                            format!(
                                                "Missing argument .insert({:?}, {:?})",
                                                key,
                                                value
                                            )
                                        )
                                    );
                                }
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument .insert({:?})", value)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .insert()")
                                )
                            );
                        }
                    }
                    Functions::Remove => {
                        // Remove takes one argument (key) and the target

                        if let Some(key) = stack.pop() {
                            if let Some(target) = stack.pop() {
                                let node = ASTNode::Function(
                                    Functions::Remove,
                                    vec![Box::new(target), Box::new(key)]
                                );
                                ast_vec.push(node.clone());
                                stack.push(node);
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument .remove({:?})", key)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .remove()")
                                )
                            );
                        }
                    }
                    Functions::Get => {
                        // Get takes one argument (key) and the target

                        if let Some(key) = stack.pop() {
                            if let Some(target) = stack.pop() {
                                let node = ASTNode::Function(
                                    Functions::Get,
                                    vec![Box::new(target), Box::new(key)]
                                );
                                ast_vec.push(node.clone());
                                stack.push(node);
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument .get({:?})", key)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .get()")
                                )
                            );
                        }
                    }
                    Functions::Assign => {
                        // Assign takes a variable name and a value as parameters

                        if let Some(arg_1) = stack.pop() {
                            if let Some(arg_0) = stack.pop() {
                                let node = ASTNode::Function(
                                    Functions::Assign,
                                    vec![Box::new(arg_0), Box::new(arg_1)]
                                );
                                ast_vec.push(node.clone());
                                stack.push(node);
                            } else {
                                return Err(
                                    ASTError::InvalidFunctionInvocation(
                                        format!("Missing argument .assign({:?})", arg_1)
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .assign()")
                                )
                            );
                        }
                    }
                    Functions::ToLower => {
                        // ToLower takes the preceeding token
                        if let Some(arg) = stack.pop() {
                            let node = ASTNode::Function(Functions::ToLower, vec![Box::new(arg)]);
                            ast_vec.push(node.clone());
                            stack.push(node);
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .tolower()")
                                )
                            );
                        }
                    }
                    Functions::ToUpper => {
                        // ToUpper takes the preceeding token
                        if let Some(arg) = stack.pop() {
                            let node = ASTNode::Function(Functions::ToUpper, vec![Box::new(arg)]);
                            ast_vec.push(node.clone());
                            stack.push(node);
                        } else {
                            return Err(
                                ASTError::InvalidFunctionInvocation(
                                    format!("Missing argument .toupper()")
                                )
                            );
                        }
                    }
                    Functions::Custom => {
                        // Not known in the beginning.
                        // First find out the target blockchain
                        // Get to the endpoint which is a valid path to the connection.json
                        let mut args_node: Vec<ASTNode> = vec![];

                        while !stack.is_empty() {
                            let arg = stack.pop().unwrap();
                            args_node.push(arg.clone());

                            match arg.evaluate() {
                                Ok(a) => {
                                    // Check if is target
                                    let f = format!("functions/{}/connection.json", a.get_value());
                                    let p = Path::new(&f);
                                    if p.exists() {
                                        let contents = fs
                                            ::read_to_string(p)
                                            .expect("File not found or unable to read file");
                                        let json: Value = serde_json
                                            ::from_str(&contents)
                                            .expect("JSON was not well-formatted");
                                        let endpoint = json["endpoint"].as_str().unwrap();

                                        args_node.reverse();

                                        let args = args_node
                                            .iter()
                                            .map(|x| Box::new(x.clone()))
                                            .collect();

                                        let node = ASTNode::Function(Functions::Custom, args);
                                        ast_vec.push(node.clone());
                                        stack.push(node);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    args_node.reverse();
                                    let args: Vec<Box<ASTNode>> = args_node
                                        .iter()
                                        .map(|x| Box::new(x.clone()))
                                        .collect();
                                    match args_node[1].evaluate() {
                                        Ok(a) => {
                                            return Err(
                                                ASTError::InvalidCustomCall(
                                                    args_node[1].evaluate().unwrap().get_value(),
                                                    e.to_string()
                                                )
                                            );
                                        }
                                        Err(e) => {
                                            return Err(
                                                ASTError::InvalidCustomCall(
                                                    "Custom".to_string(),
                                                    e.to_string()
                                                )
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        if stack.is_empty() {
                            // Stack is empty and no valid path found
                            return Err(ASTError::InvalidFunction("call()".to_string()));
                        }
                    }
                    Functions::Require => {
                        if let Some(arg_1) = stack.pop() {
                            if let Some(arg_0) = stack.pop() {
                                println!("Arg 0: {:?}", arg_0);
                                println!("Arg 1: {:?}", arg_1);
                                let node = ASTNode::Function(
                                    Functions::Require,
                                    vec![Box::new(arg_0), Box::new(arg_1)]
                                );
                                ast_vec.push(node.clone());
                                stack.push(node);
                            } else {
                                return Err(
                                    ASTError::MissingArgument(
                                        "require".to_string(),
                                        "lhs can not be empty".to_string()
                                    )
                                );
                            }
                        } else {
                            return Err(
                                ASTError::MissingArgument(
                                    "require".to_string(),
                                    "rhs can not be empty".to_string()
                                )
                            );
                        }
                    }
                }
            } else {
                // Parse array
                if token.starts_with('[') {
                    is_array = true;
                    continue;
                }

                if token.ends_with(']') {
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
                let node = match parse_token(token.clone()) {
                    Ok(node) => {
                        ast_vec.push(node.clone());
                        stack.push(node);
                    }
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

pub fn parse_token(token: String) -> Result<ASTNode, &'static str> {
    match token.parse::<u256>() {
        Ok(value) => Ok(ASTNode::ConstantNumber(value)),
        Err(_) => {
            //println!("{} is not a number", token);
            match token.parse::<bool>() {
                Ok(value) => Ok(ASTNode::ConstantBool(value)),
                Err(_) =>
                    match token.parse::<i256>() {
                        Ok(value) => Ok(ASTNode::ConstantSignedNumber(value)),
                        Err(_) => {
                            if token.starts_with('$') {
                                Ok(ASTNode::Variable(token[1..].to_string()))
                            } else {
                                Ok(ASTNode::ConstantString(token))
                            }
                        }
                    }
            }
        }
    }
}

/// The shunting yard algorithm by Dijkstra transforms the infix logic expression into postfix.
pub fn shunting_yard_algorithm(tokens: Vec<String>) -> Result<VecDeque<String>, &'static str> {
    let mut stack: Vec<String> = vec![]; // Stack for operators
    let mut output_queue: VecDeque<String> = VecDeque::new();

    for token in tokens.iter() {
        // println!("Stack: {:?}", stack);
        // println!("Output Queue: {:?}", output_queue);
        // println!("Token: {}", token);
        // Put functions on the Stack
        if Functions::from_str(token).is_ok() {
            stack.push(token.clone());
            continue;
        }

        if is_left_parenthesis(token) {
            stack.push(token.clone());
            continue;
        }

        if is_right_parenthesis(token) {
            while !is_left_parenthesis(stack.last().unwrap()) {
                if stack.is_empty() {
                    return Err("Unmatched Parentheses: Empty Stack. No preceeding parentheses");
                }
                output_queue.push_back(stack.pop().unwrap());
            }
            stack.pop(); // Remove parenthesis
            if
                Functions::from_str(
                    stack.last().unwrap_or(&"Stack seems to be empty".to_string())
                ).is_ok()
            {
                output_queue.push_back(stack.pop().unwrap());
            }
            continue;
        }

        if token == "," {
            while !is_left_parenthesis(stack.last().unwrap()) {
                output_queue.push_back(stack.pop().unwrap());
            }
            continue;
        }

        if is_operator(token) {
            while stack.last().is_some() {
                if is_left_parenthesis(stack.last().unwrap()) {
                    break;
                }
                if operator_precedence(stack.last().unwrap()) >= operator_precedence(token) {
                    output_queue.push_back(stack.pop().unwrap());
                }
            }
            stack.push(token.clone());
            continue;
        }

        // println!("Pushing to output queue: {}", token);
        // Put tokens on the Output Queue
        output_queue.push_back(token.clone());

        //     // All Operators are left
        //     match token.as_str() {
        //         "(" => {
        //             stack.push(token.to_string());
        //         },
        //         ")" => {
        //             while stack.last() != Some(&"(".to_owned()) {
        //                 output_queue.push_back(stack.pop().unwrap());
        //                 if stack.is_empty() {
        //                     return Err("Unmatched Parentheses: Empty Stack. No preceeding parentheses");
        //                 }
        //             }
        //             stack.pop(); // Remove parenthesis
        //         },
        //         _ => {

        //             while !stack.is_empty() {
        //                 if stack.last().unwrap() == "(" {
        //                     break;
        //                 }
        //                 let op_precedence = operator_precedence(stack.last().unwrap());
        //                 let self_precedence = operator_precedence(token);
        //                 //println!("op_precedence: {:?}, self_precedence: {:?}", op_precedence, self_precedence);
        //                 if op_precedence >= self_precedence {
        //                     output_queue.push_back(stack.pop().unwrap());
        //                 }else{
        //                     break;
        //                 }
        //             }
        //             stack.push(token.to_string());
        //         }
        //     }
    }

    while !stack.is_empty() {
        // println!("Stack: {:?}", stack);
        let val = stack.pop().unwrap();
        if val == "(" {
            return Err(
                "Unmatched Parentheses: Leftover on Stack. More parentheses are opening than closing"
            );
        }
        output_queue.push_back(val);
    }
    // println!("Output Queue: {:?}", output_queue);

    Ok(output_queue)
}

/// Return the precedence level of the operator
fn operator_precedence(operator: &str) -> Option<u8> {
    match operator {
        "||" => Some(0), // Or
        "&&" => Some(1), // And
        "==" | "!=" => Some(2), // Equality
        "<" | ">" | "<=" | ">=" => Some(3), // Comparison
        "+" | "-" => Some(4), // Addition, Subtraction
        "*" | "/" | "%" => Some(5), // Multiplication, Division, and Modulo
        "!" | "neg" => Some(6), // Unary Operators
        "(" | ")" | "[" | "]" | "{" | "}" => Some(7), // Parentheses and Brackets for Functions and arrays
        _ => None, // No precedence for other operators
    }
}

/// Check if token is left parenthesis
fn is_left_parenthesis(token: &str) -> bool {
    token == "(" || token == "[" || token == "{"
}

/// Check if token is right parenthesis
fn is_right_parenthesis(token: &str) -> bool {
    token == ")" || token == "]" || token == "}"
}

/// Is the token an Operator
fn is_operator(token: &str) -> bool {
    token == "+" ||
        token == "-" ||
        token == "*" ||
        token == "/" ||
        token == "%" ||
        token == "||" ||
        token == "&&" ||
        token == "==" ||
        token == "!=" ||
        token == "!" ||
        token == "<" ||
        token == "<=" ||
        token == ">" ||
        token == ">=" ||
        token == "(" ||
        token == ")" ||
        token == "neg"
}

fn replace_args_in_json(json: &mut Value, args: &mut Vec<String>) {
    match json {
        Value::Array(arr) => {
            for i in arr {
                replace_args_in_json(i, args);
            }
        }
        Value::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            for k in keys {
                let v = map.get_mut(&k).unwrap();
                replace_args_in_json(v, args);
            }
        }
        Value::String(s) => {
            if s.starts_with("$") {
                if let Some(arg) = args.get(0) {
                    *s = arg.clone();
                    args.remove(0);
                }
            }
        }
        _ => {}
    }
}

fn replace_args_in_str(json: &str, args: &Vec<String>) -> Value {
    let mut json_string = json.to_string();
    let mut args_iter = args.iter();
    println!("JSON: {}", json_string);
    println!("Args: {:#?}", args);
    let re = regex::Regex::new(r"\$[a-zA-Z0-9_]*").unwrap();
    for cap in re.find_iter(&json_string.clone()) {
        if let Some(arg) = args_iter.next() {
            json_string = json_string.replace(cap.as_str(), arg);
        }
    }
    println!("{:?}", json_string);
    serde_json::from_str(&json_string).unwrap()
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RPCRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<Value>,
    pub id: String,
}

impl std::fmt::Display for RPCRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RPCRequest {{ jsonrpc: {}, method: {}, params: {:?}, id: {} }}",
            self.jsonrpc,
            self.method,
            self.params,
            self.id
        )
    }
}

fn replace_args_in_value(json: &mut RPCRequest, args: &Vec<String>) -> Result<(), ASTError> {
    if args.is_empty() {
        return Err(ASTError::RequestReplacementError(format!("{}", json)));
    }

    match args.get(0) {
        Some(arg) => {
            json.method = arg.clone();
        }
        None => {
            return Err(ASTError::RequestReplacementError(format!("{}", json)));
        }
    }
    let remaining_args = args[1..].to_vec();
    for arg in remaining_args {
        json.params.push(parse_string(&arg));
    }
    // match args.get(1) {
    //     Some(arg) => {
    //         let v = parse_string(arg);            
    //         json.params = vec![v];
    //     }
    //     None => {
    //         return Err(ASTError::RequestReplacementError(format!("{}", json)));
    //     }
    // }

    Ok(())
}

fn parse_string(arg: &str) -> Value {
    if let Ok(v) = serde_json::from_str(arg) {
        return v;
    }
    if arg.starts_with("[") {
        return serde_json::from_str(&arg).unwrap();
    }
    if arg.starts_with("{") {
        return serde_json::from_str(&arg).unwrap();
    }
    let re = regex::Regex::new(r"\w*").unwrap();
    if re.is_match(&arg) {
        let new_arg = format!("\"{}\"", arg);
        return serde_json::from_str(&new_arg).unwrap();
    }
    serde_json::Value::Null
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
    let mut state = 0; // 0 is outside function above is inside function depth

    let mut is_array = false;
    for c in text.chars() {
        match c {
            ' ' | ',' | '.' => {
                if !current_token.is_empty() && !is_array {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            '(' | '[' | '{' => {
                if let Ok(f) = Functions::from_str(&current_token.clone()) {
                    state += 1;
                    tokens.push(current_token.clone());
                    current_token.clear();
                } else {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
                current_token.push(c);
                tokens.push(current_token.clone());
                current_token.clear();
            }
            ')' | ']' | '}' => {
                if state != 0 {
                    state -= 1;
                }
                tokens.push(current_token.clone());
                current_token.clear();
                current_token.push(c);
                tokens.push(current_token.clone());
                current_token.clear();
            }
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
pub fn build_ast_root(text: &str) -> Result<ASTNode, &'static str> {
    let tokens = tokenize(text.to_string());

    match shunting_yard_algorithm(tokens) {
        Ok(postfix) => {
            // println!("{:?}", postfix);
            match parse_postfix(postfix) {
                Ok((_, root)) => {
                    return Ok(root);
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                    return Err("Invalid Parsing of Postfix");
                }
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
            return Err("Invalid Shunting Yard Algorithm");
        }
    }
}

/// Build an AST and return each root
pub fn build_code(text: &str) -> Result<Vec<ASTNode>, &'static str> {
    let tokens = tokenize(text.to_string());

    let mut all_statements: Vec<Vec<String>> = vec![];

    let mut stmt: Vec<String> = vec![];
    for t in tokens.iter() {
        if t == "\n" {
            if stmt.is_empty() {
                continue;
            }
            all_statements.push(stmt.clone());
            stmt.clear();
            continue;
        }
        stmt.push(t.clone());
    }

    let mut code = vec![];
    for (line, stmt) in all_statements.iter().enumerate() {
        match shunting_yard_algorithm(stmt.clone()) {
            Ok(postfix) => {
                // println!("{:?}", postfix);
                match parse_postfix(postfix) {
                    Ok((_, root)) => code.push(root),
                    Err(e) => {
                        println!("Error in line {}: {:?}", line, e);
                        return Err("Invalid Parsing of Postfix");
                    }
                }
            }
            Err(e) => {
                println!("Error in line {}: {:?}", line, e);
                return Err("Invalid Shunting Yard Algorithm");
            }
        }
    }
    return Ok(code);
}

pub fn build_ast(text: &str) -> Result<(Vec<ASTNode>, ASTNode), &'static str> {
    let tokens = tokenize(text.to_string());
    if let Ok(postfix) = shunting_yard_algorithm(tokens) {
        if let Ok((ast, root)) = parse_postfix(postfix) {
            Ok((ast, root))
        } else {
            Err("Invalid AST")
        }
    } else {
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
            let buf = n.to_be_bytes();
            let start = (if in_array { 0 } else { 32 - u256::leading_zeros(n.to_be()) }) as usize;
            out.extend_from_slice(&buf[start..32]);
        }
        ASTConstant::SignedNumber(n) => {
            let buf = n.to_be_bytes();
            let start = (if in_array { 0 } else { 32 - i256::leading_zeros(n.to_be()) }) as usize;
            out.extend_from_slice(&buf[start..32]);
        }
        ASTConstant::Bool(b) => {
            if in_array {
                out.extend_from_slice(&[0; 31]);
            }
            out.push(*b as u8);
        }
        ASTConstant::String(s) => {
            if s.starts_with("0x") && s.len() == 42 {
                // Address
                if in_array {
                    out.extend_from_slice(&[0; 12]);
                }
                out.extend_from_slice(&s.as_bytes());
            } else {
                out.extend_from_slice(s.as_bytes());
            }
        }
        ASTConstant::Array(vec) => {
            for t in vec {
                encode_token(t, out, true);
            }
        }
        _ => {}
    }
}

fn max_encoded_length(t: &ASTConstant) -> usize {
    match t {
        ASTConstant::Number(_) | ASTConstant::SignedNumber(_) => 32,
        ASTConstant::String(s) => {
            if s.starts_with("0x") && s.len() == 42 {
                // Address
                20
            } else {
                s.len()
            }
        }
        ASTConstant::Bool(b) => 1,
        ASTConstant::Array(vec) =>
            vec
                .iter()
                .map(|x| max_encoded_length(x).max(32))
                .sum(),
        _ => 32,
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
mod test_ast {
    use std::hash::Hash;

    use ethnum::AsU256;
    use sha3::digest::typenum::SquareRoot;

    use crate::properties::{ ast::*, environment::print_variables };

    #[test]
    fn test_tokenizer() {
        // let text = "$event_data.slice(0, 64) > 0 && $event_data.slice(0, 64) < 100".to_string();
        let text = "call(ethereum).call_method(abc, def, 145).data".to_string();
        let tokens = tokenize(text.clone());
        println!("{:?}", tokens);

        let test = text
            .split(" ")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        println!("{:?}", test);
    }

    #[test]
    fn test_ast() {
        // Example: 5 + 5 > 17 - 15

        let exp_a = ASTNode::ConstantNumber((5).as_u256());
        let exp_b = ASTNode::ConstantNumber((5).as_u256());
        let exp_a_b = ASTNode::BinaryArithmetic(
            ArithmeticOperator::Add,
            Box::new(exp_a),
            Box::new(exp_b)
        );

        let exp_c = ASTNode::ConstantNumber((17).as_u256());
        let exp_d = ASTNode::ConstantNumber((15).as_u256());
        let exp_c_d = ASTNode::BinaryArithmetic(
            ArithmeticOperator::Subtract,
            Box::new(exp_c),
            Box::new(exp_d)
        );

        let exp_g = ASTNode::BinaryLogic(
            LogicOperator::Greater,
            Box::new(exp_a_b),
            Box::new(exp_c_d)
        );

        let val = exp_g.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("{}: {}", const_type, value);

        assert_eq!(value, "true");
    }

    #[test]
    fn test_ast_macro() {
        let root = build_ast_root("5 == 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("{}: {}", const_type, value);
    }

    #[test]
    fn test_shunting_yard() {
        // 5 + 5 > 17 - (5 - neg 10)
        let tokens = vec![
            "keccak256",
            "(",
            "10",
            "14",
            "3",
            "hello",
            ")",
            "*",
            "(",
            "15",
            "+",
            "5",
            ")",
            "as",
            "(",
            "hex",
            ")",
            "-",
            "0xff"
        ];
        let tokens_str: Vec<String> = tokens
            .iter()
            .map(|x| x.to_string())
            .collect();
        //let tokens: Vec<String> = vec!["5".to_owned(), ">".to_owned(), "(".to_owned(), "6".to_owned(), "+".to_owned(), "5".to_owned(), ")".to_owned()];

        let output = shunting_yard_algorithm(tokens_str);
        let output = output.unwrap();
        println!("Output: {:?}", output);
        for o in output.iter() {
            print!("{}", o);
        }

        let (ast, root) = parse_postfix(output).unwrap();

        println!("Root: {}", root.format());

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();

        assert_eq!(value, "true");
    }

    #[test]
    fn test_all_operations() {
        // Greater
        let root = build_ast_root("5 > 4").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Greater: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // Less
        let root = build_ast_root("3 < 4").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Less: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // GreaterOrEqual
        let root = build_ast_root("5 >= 4").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("GreaterOrEqual: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // LessOrEqual
        let root = build_ast_root("4 <= 4").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("LessOrEqual: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // Equal
        let root = build_ast_root("5 == 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Equal: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");

        // NotEqual
        let root = build_ast_root("5 != 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("NotEqual: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "false");

        // Not
        let root = build_ast_root("! true").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Not: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "false");

        // Add
        let root = build_ast_root("5 + 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Add: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "10");

        // Subtract
        let root = build_ast_root("5 - 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Subtract: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "0");

        // Multiply
        let root = build_ast_root("5 * 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Multiply: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "25");

        // Divide
        let root = build_ast_root("5 / 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Divide: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "1");

        // Modulo
        let root = build_ast_root("5 % 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Modulo: {}: {}", const_type, value);
        assert_eq!(const_type, "Number");
        assert_eq!(value, "0");

        // Negate
        let root = build_ast_root("neg 5").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("Negate: {}: {}", const_type, value);
        assert_eq!(const_type, "SignedNumber");
        assert_eq!(value, "-5");
    }

    #[test]
    fn test_complex_ast() {
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
        let (const_type, value) = val.get_constant_info();
        println!("Add Greater: {}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");
    }

    #[test]
    fn test_more_complex_ast() {
        let root = build_ast_root("( 17 * 3 ) % 10 == 1").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("{}: {}", const_type, value);
        assert_eq!(const_type, "Bool");
        assert_eq!(value, "true");
    }

    #[test]
    fn test_variables() {
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
    fn test_str_var() {
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
    fn test_function_at() {
        set_var!("arr", "[0,1,2,3]");

        let root = build_ast_root("$arr.at(1) == 2").unwrap();

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
    fn convert_values() {
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
        root.print("");
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "0xff");
    }

    #[test]
    fn test_string_arithmetic() {
        let root = build_ast_root("0xff.as(u256) - 1").unwrap();

        let val = root.evaluate().unwrap();
        let (const_type, value) = val.get_constant_info();
        println!("{}: {}", const_type, value);
    }

    #[test]
    fn test_variable_conversion() {
        set_var!("a", "255");
        let root = build_ast_root("( $a - 1 ).as(hex)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "0xfe");
    }

    #[test]
    fn test_variable_conversion_leading_zeros() {
        set_var!("a", "0a0255");
        let root = build_ast_root("$a.as(hex)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "0x0a0255");
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
    fn test_keccak256() {
        let root = build_ast_root(
            "keccak256(2, 0xe5752128B13c709d2A7E5348E601a016136a3F28, 1000000000000000000)"
        ).unwrap();

        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "0xda907f151946daa4671efcfbd42543ab19dd868bd4f6ab3e7d330746c7683c39");
    }

    #[test]
    fn test_print() {
        let root = build_ast_root("5 + 5 == ( 15 - 5 )").unwrap();
        root.print("");
        let root = build_ast_root("( $var.push(15) && true ) || 16 / 4 > 3").unwrap();
        root.print("");
        let root = build_ast_root("[ 0, 1, 2, 3, 4, hello ]").unwrap();
        root.print("");
    }

    #[test]
    fn test_push_array_to_array() {
        set_var!("keystore", "[[1,2], [3,4]]");

        let root = build_ast_root("keystore.push([ key, value ])").unwrap();
        // root.print("");
        let ret = root.evaluate().unwrap();
        println!("{:?}", ret);

        println!("{:?}", get_variable_map_instance());
    }

    #[test]
    fn test_ast_map() {
        set_var!("map", VarValues::Map(HashMap::new()));

        println!("{:?}", get_variable_map_instance());

        let root = build_ast_root("$map.insert(aircraft, 0x12345)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);

        assert_eq!(ret, "true");
        println!("{:?}", get_variable_map_instance());

        let root = build_ast_root("$map.get(aircraft)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);

        assert_eq!(ret, "0x12345");
        println!("{:?}", get_variable_map_instance());

        let root = build_ast_root("$map.remove(aircraft)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);

        assert_eq!(ret, "0x12345");
        println!("{:?}", get_variable_map_instance());
    }

    #[test]
    fn test_map_variables() {
        set_var!("map", VarValues::Map(HashMap::new()));

        println!("{:?}", get_variable_map_instance());

        set_var!("some_key", "0x123456");
        let root = build_ast_root("$map.insert($some_key, some_value)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);

        assert_eq!(ret, "true");

        println!("{:?}", get_variable_map_instance());

        let root = build_ast_root("$map.get(0x123456)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "some_value");
    }

    #[test]
    fn test_nested_function() {
        let root = build_ast_root("hello.push(_world.push(1))").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);

        assert_eq!(ret, "hello_world1");
    }

    #[test]
    fn test_push_to_number() {
        set_var!("num", 1000);
        let root = build_ast_root("$num.push(a).push(b).as(hex)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);

        assert_eq!(ret, "0x1000ab");
    }

    #[test]
    fn test_pattern() {
        set_var!("map", VarValues::Map(HashMap::new()));

        let patt =
            "$map.insert($verified_account, $proof) && $map.insert($verified_account.push(1), $value)";

        let root = build_ast_root(patt).unwrap();
        root.print("");
    }

    #[test]
    fn test_assign() {
        let root = build_ast_root("assign(var, 15512)").unwrap();
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "true");

        println!("{:?}", get_var!("var").unwrap());
    }

    #[test]
    fn test_contains_var() {
        set_var!("var", "hello");
        set_var!("var2", "he");

        let root = build_ast_root("$var.contains($var2)").unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        let ret = val.get_value();
        println!("{}", ret);
        assert_eq!(ret, "true");
    }

    #[test]
    fn test_contains_keystore() {
        set_var!("keystore", VarValues::Array(vec![]));
        let root = build_ast_root(
            "$keystore.push(0x071b8f8f375A1932BAAF356BcA98aAC0128bf5bf)"
        ).unwrap();
        let val = root.evaluate().unwrap();

        set_var!("address", "0x071b8f8f375A1932BAAF356BcA98aAC0128bf5bf");

        let new_command = build_ast_root("$keystore.contains($address) == false").unwrap();
        let new_val = new_command.evaluate().unwrap();
        let new_ret = new_val.get_value();
        new_command.print("");
        println!("{:?}", get_variable_map_instance());
        println!("{}", new_ret);
        assert_eq!(new_ret, "false");
    }

    #[test]
    fn test_to_lower() {
        set_var!("var", "hElLo");
        let root = build_ast_root("$var.toLower()").unwrap();
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());
        assert_eq!(val.get_value(), "hello");

        let new_command = build_ast_root("$var.toUpper()").unwrap();
        let new_val = new_command.evaluate().unwrap();
        println!("{}", new_val.get_value());
        assert_eq!(new_val.get_value(), "HELLO");
    }

    #[test]
    fn test_map_get() {
        let mut map: HashMap<String, VarValues> = HashMap::new();

        map.insert("programId".to_string(), "Fg1PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS".into());
        map.insert("data".to_string(), "p3wW1xWC25pYgYq1pcMV2Ei1dL".into());
        map.insert(
            "accounts".to_string(),
            [
                "5aAj94L1nFJSDhYGSk4eQuU9QTECAAmyY8uRAHiLQty8",
                "CCBmdp8sgzxRveNYNDH28ryDqxYshZRkNHbAnGmFLKBK",
                "HPGi8BkURt5FKTpni2HNh5b2Foa3yDhSBGwJvjdLrACk",
            ].into()
        );

        set_var!("map", VarValues::Map(map));

        print_variables(&get_variable_map_instance());

        let root = build_ast_root("$map.get(accounts)").unwrap();
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        print_variables(&get_variable_map_instance());

        let root = build_ast_root("$map.insert(programId, AAAAAAAAAAAAAAAA)").unwrap();
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        print_variables(&get_variable_map_instance());
    }

    #[test]
    fn test_return_array() {
        let mut map: HashMap<String, VarValues> = HashMap::new();

        map.insert("arr".to_string(), "[1,2,3,4,5,6]".into());
        set_var!("map", VarValues::Map(map));

        let root = build_ast_root("$map.get(arr)").unwrap();
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());
    }

    #[test]
    fn test_custom_function() {
        let root = build_ast_root(
            "call(ethereum, eth_getBalance, [0xa58A9d3A5E240b09Da3Bc0BFc011AF3d20D31763, latest]).get(result)"
        ).unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        print_variables(&get_variable_map_instance());
    }

    #[test]
    fn test_replace_args() {
        let root = build_ast_root(
            "call(ethereum, get_logs, [\"0x97116cf6cd4f6412bb47914d6db18da9e16ab2142f543b86e207c24fbd16b23a\",\"0xdbb69440df8433824a026ef190652f29929eb64b4d1d5d2a69be8afe3e6eaed8\",\"0xa67d828453163879637ade5a7d51abb746669dbc34d7e2149e8fec3bf71fff54\"]).get(result)"
        ).unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        print_variables(&get_variable_map_instance());
    }

    #[test]
    fn test_remove_arr() {
        set_var!("arr", VarValues::Array(vec![]));

        let root = build_ast_root("$arr.push(14)").unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        let root = build_ast_root("$arr.push(1000)").unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        let root = build_ast_root("$arr.push(4056)").unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        let root = build_ast_root("$arr.remove(14)").unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        println!("{:?}", get_var!("arr"));
    }

    #[test]
    fn test_full_code() {
        let root = build_code(
            "
            assign(stuff, call(ethereum, get_balance, [0xa58A9d3A5E240b09Da3Bc0BFc011AF3d20D31763, latest]))
            $stuff.get(result)
        "
        ).unwrap();

        for r in root {
            let val = r.evaluate().unwrap();
            println!("{}", val.get_value());
        }
    }

    #[test]
    fn test_insert_remove_map() {
        let mut map: HashMap<String, VarValues> = HashMap::new();
        set_var!("map", VarValues::Map(map));
        let root = build_ast_root(
            "$map.insert(0xe5752128B13c709d2A7E5348E601a016136a3F28, 0xda907f151946daa4671efcfbd42543ab19dd868bd4f6ab3e7d330746c7683c39) && ($map.remove(0xe5752128B13c709d2A7E5348E601a016136a3F28) == 0xda907f151946daa4671efcfbd42543ab19dd868bd4f6ab3e7d330746c7683c39)"
        ).unwrap();

        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());
    }

    #[test]
    fn test_require() {
        let mut map: HashMap<String, VarValues> = HashMap::new();
        set_var!("map", VarValues::Map(map));
        build_ast_root(
            "$map.insert(0xa58a9d3a5e240b09da3bc0bfc011af3d20d31763,0xf8e81d47203a594245e36c48e151709f0c19fbe8)"
        )
            .unwrap()
            .evaluate()
            .unwrap();
        let root = build_ast_root(
            "require(($map.get(0xa58a9d3a5e240b09da3bc0bfc011af3d20d31763) == 0xf8e81d47203a594245e36c48e151709f0c19fbe8), $map.remove(0xa58a9d3a5e240b09da3bc0bfc011af3d20d31763) == 0xf8e81d47203a594245e36c48e151709f0c19fbe8)"
        ).unwrap();
        let val = root.evaluate().unwrap();
        println!("{}", val.get_value());

        print_variables(&get_variable_map_instance());
    }
}
