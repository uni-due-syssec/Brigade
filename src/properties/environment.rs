use std::collections::HashMap;

use std::mem::MaybeUninit;
use std::str::FromStr;
use std::sync::Once;

use crate::utils;

use super::ast::ASTNode;

use ethnum::{u256, i256, int, uint, AsI256, AsU256, U256, I256};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum VarValues{
    String(String),
    Number(u256),
    SignedNumber(i256),
    Bool(bool),
    Array(Vec<VarValues>),
}

pub trait GetVar<T> {
    fn get_value(value: VarValues) -> Option<T>;
}

impl GetVar<i32> for i32{
    fn get_value(value: VarValues) -> Option<i32> {
        if let VarValues::Number(v) = value {
            if v < i32::MAX.as_u256() {
                Some(v.as_i32())    
            }else{
                None
            }
        }else{
            if let VarValues::SignedNumber(v) = value {
                return Some(v.as_i32());
            }
            None
        }
    }
}

impl GetVar<u32> for u32{
    fn get_value(value: VarValues) -> Option<u32> {
        if let VarValues::Number(v) = value {
            Some(v.as_u32())
        }else{
            if let VarValues::SignedNumber(v) = value {
                return Some(v.as_u32());
            }
            None
        }
    }
}

impl GetVar<i64> for i64{
    fn get_value(value: VarValues) -> Option<i64> {
        if let VarValues::Number(v) = value {
            Some(v.as_i64())
        }else{
            if let VarValues::SignedNumber(v) = value {
                return Some(v.as_i64());
            }
            None
        }
    }
}

impl GetVar<u64> for u64{
    fn get_value(value: VarValues) -> Option<u64> {
        if let VarValues::Number(v) = value {
            Some(v.as_u64())
        }else{
            if let VarValues::SignedNumber(v) = value {
                return Some(v.as_u64());
            }
            None
        }
    }
}

impl GetVar<u256> for u256{
    fn get_value(value: VarValues) -> Option<u256> {
        if let VarValues::Number(v) = value {
            Some(v)
        }else{
            if let VarValues::SignedNumber(v) = value {
                if v < 0{
                    None
                }else{
                    Some(v.as_u256())
                }
            }else{
                if let VarValues::String(v) = value {
                    if v.starts_with("u256:"){
                        let s = &v[5..];
                        Some(u256::from_str(s).unwrap())
                    }else{ 
                        None
                    }
                }else {
                    None
                }
            }
        }
    }
}

impl GetVar<i256> for i256{
    fn get_value(value: VarValues) -> Option<i256> {
        if let VarValues::SignedNumber(v) = value {
            Some(v)
        }else{
            if let VarValues::Number(v) = value {
                if v > i256::MAX.as_u256(){
                    None
                }else{
                    Some(v.as_i256())
                }
            }else{
                if let VarValues::String(v) = value {
                    if v.starts_with("i256:"){
                        let s = &v[5..];
                        Some(i256::from_str(s).unwrap())
                    }else{ 
                        None
                    }
                }else {
                    None
                }
            }
        }
    }
}

impl GetVar<bool> for bool{
    fn get_value(value: VarValues) -> Option<bool> {
        if let VarValues::Bool(v) = value {
            Some(v)
        }else{
            None
        }
    }
}

impl GetVar<String> for String{
    fn get_value(value: VarValues) -> Option<String> {
        if let VarValues::String(v) = value {
            Some(v)
        }else{
            None
        }
    }
}

// impl GetVar for Vec<VarValues>{
//     type T = Vec<VarValues>;
//     fn get_value(value: VarValues) -> Option<Vec<VarValues>> {
//         if let VarValues::Array(v) = value {
//             Some(v)
//         }else{
//             None
//         }
//     }
// }

impl<V> GetVar<Vec<V>> for Vec<V>
where
    V: GetVar<V>,
{
    fn get_value(value: VarValues) -> Option<Self> {
        if let VarValues::Array(v) = value {
            //let arr = v.iter().map(|x| x.get_value()).collect::<Vec<V>>();
            let mut arr = vec![];

            for l in v{
                arr.push(V::get_value(l).unwrap());
            }
            
            Some(arr)
        }else{
            None
        }
    }
}

impl VarValues {

    pub fn get_type(&self) -> String {
        match self {
            VarValues::String(_) => "String".to_string(),
            VarValues::Number(_) => "Number".to_string(),
            VarValues::SignedNumber(_) => "SignedNumber".to_string(),
            VarValues::Bool(_) => "Bool".to_string(),
            VarValues::Array(_) => "Array".to_string(),
        }
    }

    pub fn get_value(&self) -> String {
        match self {
            VarValues::String(value) => value.clone(),
            VarValues::Number(value) => value.to_string(),
            VarValues::SignedNumber(value) => value.to_string(),
            VarValues::Bool(value) => value.to_string(),
            VarValues::Array(value) => {
                let s = value.iter().map(|value| value.get_value()).collect::<Vec<String>>().join(",");
                return format!("[{}]", s);
            },
        }
    }

    pub fn get_string(&self) -> Option<&String> {
        match self {
            VarValues::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn get_number(&self) -> Option<u256> {
        match self {
            VarValues::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn get_signed_number(&self) -> Option<i256> {
        match self {
            VarValues::SignedNumber(value) => Some(*value),
            _ => None,
        }
    }

    pub fn get_bool(&self) -> Option<bool> {
        match self {
            VarValues::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn get_array(&self) -> Option<&Vec<VarValues>> {
        match self {
            VarValues::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn to_ASTNode(&self) -> ASTNode {
        match self {
            VarValues::String(value) => ASTNode::ConstantString(value.clone()),
            VarValues::Number(value) => ASTNode::ConstantNumber(*value),
            VarValues::SignedNumber(value) => ASTNode::ConstantSignedNumber(*value),
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
        match s.parse::<u256>() {
            Ok(value) => Ok(VarValues::Number(value)),
            Err(_) => {
                match s.parse::<i256>() {
                    Ok(value) => Ok(VarValues::SignedNumber(value)),
                    Err(_) => match s.parse::<bool>() {
                        Ok(value) => Ok(VarValues::Bool(value)),
                        Err(_) => Ok(VarValues::String(s.to_owned())),
                    }
                }
            }
        }
    }
}

impl From<Vec<VarValues>> for VarValues {
    fn from(v: Vec<VarValues>) -> Self {
        VarValues::Array(v)
    }
}

impl From<Vec<u256>> for VarValues {
    fn from(v: Vec<u256>) -> Self {
        VarValues::Array(v.iter().map(|x| VarValues::from(*x)).collect())
    }
}

impl From<Vec<i256>> for VarValues {
    fn from(v: Vec<i256>) -> Self {
        VarValues::Array(v.iter().map(|x| VarValues::from(*x)).collect())
    }
}

impl From<Vec<u64>> for VarValues {
    fn from(v: Vec<u64>) -> Self {
        let arr = v.iter().map(|x| VarValues::from(*x)).collect();
        VarValues::Array(arr)
    }
}

impl From<Vec<i64>> for VarValues {
    fn from(v: Vec<i64>) -> Self {
        let arr = v.iter().map(|x| VarValues::from(*x)).collect();
        VarValues::Array(arr)
    }
}

impl From<Vec<u32>> for VarValues {
    fn from(v: Vec<u32>) -> Self {
        let arr = v.iter().map(|x| VarValues::from(*x)).collect();
        VarValues::Array(arr)
    }
}

impl From<Vec<i32>> for VarValues {
    fn from(v: Vec<i32>) -> Self {
        let arr = v.iter().map(|x| VarValues::from(*x)).collect();
        VarValues::Array(arr)
    }
}

impl From<Vec<bool>> for VarValues {
    fn from(v: Vec<bool>) -> Self {
        let arr = v.iter().map(|x| VarValues::from(*x)).collect();
        VarValues::Array(arr)
    }
}

impl From<Vec<String>> for VarValues {
    fn from(v: Vec<String>) -> Self {
        let arr = v.iter().map(|x| VarValues::from(x.clone())).collect();
        VarValues::Array(arr)
    }
}

impl From<String> for VarValues {
    fn from(s: String) -> Self {
        VarValues::String(s)
    }
}

impl From<&str> for VarValues {
    fn from(s: &str) -> Self {
        VarValues::String(s.to_owned())
    }
}

impl From<bool> for VarValues {
    fn from(s: bool) -> Self {
        VarValues::Bool(s)
    }
}

impl From<u256> for VarValues {
    fn from(s: u256) -> Self {
        VarValues::Number(s)
    }
}

impl From<i256> for VarValues {
    fn from(s: i256) -> Self {
        VarValues::SignedNumber(s)
    }
}

impl From<u64> for VarValues {
    fn from(s: u64) -> Self {
        VarValues::Number(s.as_u256())
    }
}

impl From<i64> for VarValues {
    fn from(s: i64) -> Self {
        VarValues::SignedNumber(s.as_i256())
    }
}

impl From<i32> for VarValues {
    fn from(s: i32) -> Self {
        VarValues::SignedNumber(s.as_i256())
    }
}

impl From<u32> for VarValues {
    fn from(s: u32) -> Self {
        VarValues::Number(s.as_u256())
    }
}

impl From<Value> for VarValues {
    fn from(s: Value) -> Self {
        match s {
            Value::String(s) => VarValues::String(s),
            Value::Number(s) => {
                if s.is_i64() {
                    VarValues::SignedNumber(s.as_i64().unwrap().as_i256())
                }else{
                    VarValues::Number(s.as_u64().unwrap().as_u256())
                }
            },
            Value::Bool(s) => VarValues::Bool(s),
            Value::Array(s) => {
                let mut arr = Vec::new();

                for v in s{
                    arr.push(VarValues::from(v));
                }
                VarValues::Array(arr)
            },
            _ => VarValues::String(s.to_string()),
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

impl PartialEq<u256> for VarValues {
    fn eq(&self, other: &u256) -> bool {
        match self {
            VarValues::Number(s) => *s == *other,
            _ => false,
        }
    }
}

impl PartialEq<i256> for VarValues {
    fn eq(&self, other: &i256) -> bool {
        match self {
            VarValues::SignedNumber(s) => *s == *other,
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

impl PartialOrd<u256> for VarValues {
    fn partial_cmp(&self, other: &u256) -> Option<std::cmp::Ordering> {
        match self {
            VarValues::Number(s) => s.partial_cmp(other),
            _ => None,
        }
    }
}

impl PartialOrd<i256> for VarValues {
    fn partial_cmp(&self, other: &i256) -> Option<std::cmp::Ordering> {
        match self {
            VarValues::SignedNumber(s) => s.partial_cmp(other),
            _ => None,
        }
    }
}

pub type VariableMap = HashMap<String, VarValues>;

pub fn get_variable(map: &VariableMap, key: &str) -> Option<ASTNode> {
    map.get(key).map(|v| v.to_ASTNode())
}

pub fn get_var<T: GetVar<T>>(map: &VariableMap, key: &str) -> Option<T> {
    let val = map.get(key).expect("Value not found");
    T::get_value(val.clone())
}

pub fn list_variables(map: &VariableMap) -> Vec<String> {
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
        let value: VarValues = VarValues::from($value);
        get_variable_map_instance().insert($key.to_owned(), value);
    };
}

#[macro_export]
macro_rules! get_var {
    ($key:expr) => {
        match get_variable_map_instance().get($key){
            Some(value) => Some(value.clone()),
            None => None
        }
    };

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

    ($key:expr) => {
        match get_variable_map_instance().get($key){
            Some(value) => Some(value.clone()),
            None => None
        }
    };

    (i256 $key:expr) => {
        match get_variable_map_instance().get($key){
            Some(value) => Some(i256::get_value(value.clone()).unwrap()),
            None => None
        }
    };

    (u256 $key:expr) => {
        match get_variable_map_instance().get($key){
            Some(value) => Some(u256::get_value(value.clone()).unwrap()),
            None => None
        }
    };

    (bool $key:expr) => {
        match get_variable_map_instance().get($key){
            Some(value) => Some(bool::get_value(value.clone()).unwrap()),
            None => None
        }
    };

    (String $key:expr) => {
        match get_variable_map_instance().get($key){
            Some(value) => Some(String::get_value(value.clone()).unwrap()),
            None => None
        }
    };

    (Array $key:expr) => {
        match get_variable_map_instance().get($key){
            Some(value) => Some(Vec::get_value(value.clone()).unwrap()),
            None => None
        }
    }
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
    set_var!("a", "-1");
    let a = get_var!(value "a").unwrap();

    assert_eq!(a, "-1");
}

#[test]
fn test_var_types() {
    set_var!("a", 15);
    
    println!("{:?}", get_variable_map_instance().get("a").unwrap().get_type());

    let a: I256 = get_var::<I256>(&get_variable_map_instance(), "a").unwrap();

    assert_eq!(a, 15);
}

#[test]
fn test_unknown_var() {
    assert_eq!(get_var!("unknown"), None);
}

#[test]
fn test_typing(){
    set_var!("a", 15);
    let a = get_var!(i256 "a").expect("Value not found");
    assert_eq!(a, 15);

    let b = get_var!(u256 "a").expect("Value not found");
    assert_eq!(b, 15);

    set_var!("c", true);
    let c = get_var!(bool "c").expect("Value not found");
    assert_eq!(c, true);

    set_var!("d", "String");
    let d = get_var!(String "d").expect("Value not found");
    assert_eq!(d, "String");

    let v = vec![1,2,3];
    set_var!("e", v.clone());
    let e: Vec<i32> = get_var!(Array "e").expect("Value not found");
    assert_eq!(e, v);
}