use std::{collections::HashMap, path::PathBuf, fs, str::FromStr, fmt::Debug};

use serde::{Deserialize, Serialize};


/// How should a description look like?
/// 
/// A description looks like this:
/** 
 * 
    {
        "event": "SendEthToSol",
        "chain_name": "ethereum"
        "properties": {
            "from": "get_transaction_By_Hash.from",
            "balance": "get_balance(from)",
            "balance_before": "get_balance(from)",
            "value": "get_transaction_By_Hash.value"
        },
        "pattern": [
            "balance",
            "<",
            "balance_before",
        ]
    }
 * 
 **/
/// A description file should start with the name of the event that triggers the check
/// A chain_name should be given to the file to indicate where the data should come from
/// Then some properties are defined which should be fetched from the blockchain and can be used in the pattern segment
/// The pattern should be a list of properties and logic operators that indicate which property should not be violated by transactions.

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DescriptionParser{
    pub event: String,
    #[serde(rename = "chain_name")]
    pub chain_name: String,
    pub properties: HashMap<String, String>,
    pub pattern: Vec<String>,
}

impl DescriptionParser{
    /// The Parser should take the properties and get the correct property from the property struct
    /// Then the parser should take the pattern and check which vector entry is a logic operator
    /// The other entries should be matched with the properties found in the property struct
    /// The logic operators should be parsed into the LogicParser which returns true or false depending on the values.
    /// Only if all patterns return true, the parser should return true
    pub fn parse_pattern(self) -> bool{
        false
    }
}

#[derive(Debug, Clone)]
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

/// Parse a Logical Statement and return the Boolean
#[derive(Debug, Clone)]
pub struct LogicParser<T>{
    pub val1: T,
    pub logic_operator: LogicOperator,
    pub val2: T,
}

/// Generic Implementations
impl<T> LogicParser<T>{
    /// Create a new Parser
    pub fn new(val1: T, logic_operator: LogicOperator, val2: T) -> Self {
        Self{
            val1,
            logic_operator,
            val2
        }
    }
}


/// Implementation for all Number Types
/// Numbers can be compared with ==, !=, >, <, >=, <=
impl<T> LogicParser<T> 
where
    T: PartialOrd + PartialEq + FromStr,
{


    /// Create Parser from String with space separated statement
    /// Warning: Don't use with string slices
    pub fn new_from_str(statement: &str) -> Result<Self, &'static str> {
        let mut statement = statement.split(" ");
        let val1 = match statement.next().unwrap().parse::<T>() {
            Ok(val) => val,
            Err(_) => return Err("Invalid Number Format"),
        };
        let logic_operator = LogicOperator::from_str(statement.next().unwrap()).unwrap();
        let val2 = match statement.next().unwrap().parse::<T>() {
            Ok(val) => val,
            Err(_) => return Err("Invalid Number Format"),
        };
        Ok(Self{
            val1,
            logic_operator,
            val2
        })
    }

    pub fn check(self) -> bool {
        match self.logic_operator {
            LogicOperator::Greater => {
                if self.val1 > self.val2 {
                    true
                }else{
                    false
                }
            },
            LogicOperator::Less => {
                if self.val1 < self.val2 {
                    true
                }else{
                    false
                }
            },
            LogicOperator::GreaterOrEqual => {
                if self.val1 >= self.val2 {
                    true
                }else{
                    false
                }
            },
            LogicOperator::LessOrEqual => {
                if self.val1 <= self.val2 {
                    true
                }else{
                    false
                }
            },
            LogicOperator::Equal => {
                if self.val1 == self.val2 {
                    true
                }else{
                    false
                }
            },
            LogicOperator::NotEqual => {
                if self.val1 != self.val2 {
                    true
                }else {
                    false
                }
            },
            _ => {
                println!("Invalid Operation");
                false
            }
        }
    }
}


/// Implementation for all Number Types
/// Numbers can be compared with ==, !=, >, <, >=, <=
impl LogicParser<String> 
{

    /// Create Parser from String with space separated statement
    /// Used for parsing to string slices
    pub fn to_str_parser(statement: &str) -> Result<Self, &'static str> {
        let mut statement = statement.split(" ");
        let val1 = statement.next().unwrap().to_owned();
        let logic_operator = LogicOperator::from_str(statement.next().unwrap()).unwrap();
        let val2 = statement.next().unwrap().to_owned();
        Ok(Self{
            val1,
            logic_operator,
            val2
        })
    }

    pub fn check_str(self) -> bool {
        match self.logic_operator {
            LogicOperator::Equal => {
                if self.val1 == self.val2 {
                    true
                }else{
                    false
                }
            },
            LogicOperator::NotEqual => {
                if self.val1 != self.val2 {
                    true
                }else {
                    false
                }
            },
            _ => {
                println!("Invalid Operation");
                false
            }
        }
    }
}

impl<T> LogicParser<LogicParser<T>>{
    todo!("Implement Logic Parser for recursive Statement");
}

/// Check if the Logic evaluates to true
macro_rules! parse_logic {
    (val1: T, logic_operator: LogicOperator, val2: T) => {
        assert_eq(LogicParser::new(val1, logic_operator, val2).check(), true);
    };
}

/// Check if the Logic evaluates to true
macro_rules! parse_logic_str {
    (val1: String, logic_operator: LogicOperator, val2: String) => {
        assert_eq(LogicParser::to_str_parser(val1, logic_operator, val2).unwrap().check_str(), true);
    };
}

#[test]
fn test_description_file_serialization(){
    let path = PathBuf::from("properties/property_definition.json");
    let content = fs::read_to_string(path).unwrap();
    let descriptor: DescriptionParser = serde_json::from_str(&content).unwrap();

    println!("{:?}", descriptor);
}

#[test]
fn test_logic_operator_u64(){

    let integer: u64 = 55555555;
    let integer2: u64 = 14;
    let parser: LogicParser<u64> = LogicParser{
        val1: integer, logic_operator: LogicOperator::Greater, val2: integer2
    };

    assert_eq!(parser.check(), true);
}

#[test]
fn test_logic_equality_str() {
    let string1 = "hello";
    let string2 = "hell";
    let mut parser: LogicParser<String> = LogicParser{
        val1: string1.to_owned(), logic_operator: LogicOperator::NotEqual, val2: string2.to_owned()
    };

    assert_eq!(parser.clone().check_str(), true);

    parser.logic_operator = LogicOperator::Equal;
    parser.val2 = string1.to_owned();

    assert_eq!(parser.check_str(), true);
}