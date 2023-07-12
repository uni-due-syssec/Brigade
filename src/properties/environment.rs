use std::collections::HashMap;

use std::mem::MaybeUninit;
use std::sync::Once;

pub type VariableMap = HashMap<String, String>;

pub fn get_variable(map: &VariableMap, key: &str) -> Option<String> {
    map.get(key).map(|v| v.to_string())
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

macro_rules! set_var{
    ($key:expr, $value:expr) => {
        get_variable_map_instance().insert($key.to_string(), $value.to_string());
    }
}

macro_rules! get_var {
    ($key:expr) => {
        get_variable_map_instance().get($key).unwrap().to_string()
    }
}

#[test]
fn test_static_var_map() {
    let map = get_variable_map_instance();

    map.insert("a".to_owned(), "1".to_owned());

    let test_map = get_variable_map_instance();
    let a = get_variable(test_map, "a");

    assert_eq!(a.unwrap(), "1");
}

#[test]
fn test_set_get_var() {
    set_var!("a", "1");
    let a = get_var!("a");

    assert_eq!(a, "1");
}