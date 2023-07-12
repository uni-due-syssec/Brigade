use std::collections::HashMap;

use std::mem::MaybeUninit;
use std::str::FromStr;
use std::sync::Once;

use super::ast::ASTNode;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum VarValues{
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<VarValues>),
}

impl VarValues {
    pub fn get_value(&self) -> String {
        match self {
            VarValues::String(value) => value.clone(),
            VarValues::Number(value) => value.to_string(),
            VarValues::Bool(value) => value.to_string(),
            VarValues::Array(value) => {
                let s = value.iter().map(|value| value.get_value()).collect::<Vec<String>>().join(",");
                return format!("[{}]", s);
            },
        }
    }

    pub fn to_ASTNode(&self) -> ASTNode {
        match self {
            VarValues::String(value) => ASTNode::ConstantString(value.clone()),
            VarValues::Number(value) => ASTNode::ConstantNumber(*value),
            VarValues::Bool(value) => ASTNode::ConstantBool(*value),
            VarValues::Array(value) => {
                let mut arr = Vec::new();
                for v in value {
                    arr.push(Box::new(v.to_ASTNode()));
                }
                ASTNode::Array(arr)
            },
        }
    }
}

impl FromStr for VarValues {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {

        if s.starts_with("["){
            // Is Array
            let mut arr: Vec<VarValues> = Vec::new();
            for v in s[1..s.len() - 1].split(","){
                arr.push(VarValues::from_str(v).unwrap());
            }
            return Ok(VarValues::Array(arr));
        }
        match s.parse::<f64>() {
            Ok(value) => Ok(VarValues::Number(value)),
            Err(_) => {
                match s.parse::<bool>() {
                    Ok(value) => Ok(VarValues::Bool(value)),
                    Err(_) => Ok(VarValues::String(s.to_owned())),
                }
            }
        }
    }
}

impl PartialEq<str> for VarValues {
    fn eq(&self, other: &str) -> bool {
        match self {
            VarValues::String(s) => *s == *other,
            _ => false,
        }
    }
}

impl PartialEq<f64> for VarValues {
    fn eq(&self, other: &f64) -> bool {
        match self {
            VarValues::Number(s) => *s == *other,
            _ => false,
        }
    }
}

impl PartialEq<bool> for VarValues {
    fn eq(&self, other: &bool) -> bool {
        match self {
            VarValues::Bool(s) => *s == *other,
            _ => false,
        }
    }
}

impl PartialOrd<f64> for VarValues {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        match self {
            VarValues::Number(s) => s.partial_cmp(other),
            _ => None,
        }
    }
}

pub type VariableMap = HashMap<&'static str, VarValues>;

pub fn get_variable(map: &VariableMap, key: &str) -> Option<ASTNode> {
    map.get(key).map(|v| v.to_ASTNode())
}

pub fn list_variables(map: &VariableMap) -> Vec<&'static str> {
    map.keys().cloned().collect()
}

pub fn get_variable_map_instance() -> &'static mut VariableMap {
    static mut MAYBE: MaybeUninit<VariableMap> = MaybeUninit::uninit();
    static ONLY: std::sync::Once = Once::new();

    unsafe{
        ONLY.call_once(|| {
            let var_map = VariableMap::new();
            MAYBE.write(var_map);
        });
        MAYBE.assume_init_mut()
    }
}

#[macro_export]
macro_rules! set_var{
    ($key:expr, $value:expr) => {
        let value: VarValues = VarValues::from_str($value).unwrap();
        get_variable_map_instance().insert($key, value);
    };
}

#[macro_export]
macro_rules! get_var {
    (ast_node $key:expr) => {
        match get_variable_map_instance().get($key) {
            Some(value) => Some(value.to_ASTNode()),
            None => None,
        }
    };

    (value $key:expr) => {
        match get_variable_map_instance().get($key){
            Some(value) => Some(value.get_value()),
            None => None
        }
    };
}

#[test]
fn test_static_var_map() {
    let map = get_variable_map_instance();

    set_var!("a", "1");

    let test_map = get_variable_map_instance();
    let a = get_variable(test_map, "a");

    assert_eq!(a.unwrap().evaluate().unwrap().get_value().as_str(), "1");
}

#[test]
fn test_set_get_var() {
    set_var!("a", "1");
    let a = get_var!(value "a").unwrap();

    assert_eq!(a, "1");
}