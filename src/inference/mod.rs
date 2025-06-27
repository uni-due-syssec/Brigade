use std::{
    collections::HashMap,
    fs,
    mem::MaybeUninit,
    path::Path,
    sync::{
        atomic::{self, AtomicU64},
        Once,
    },
};

use crate::{build_ast_root, ASTConstant, FEATURE_VEC_LENGTH};
// mod iso_forest;
pub(crate) mod isolation_forest;

use ethnum::{u256, U256};
use serde_json::Value;

// TODO: Maybe a trait to Ensure that all AI models implement the same functionality and can be exchanged easily
pub trait AIModel {
    fn train(&mut self);
    fn predict(&self, sample: &[u256; FEATURE_VEC_LENGTH]) -> Option<f64>;
    fn add_samples(&mut self, features: Vec<[u256; FEATURE_VEC_LENGTH]>);
    fn add_sample(&mut self, features: [u256; FEATURE_VEC_LENGTH]);
}

// TODO: Make the AIModel trait use the ModelFeature for compatibility
pub struct ModelFeature {
    pub topic: String,
    pub feature: u64,
}

impl ModelFeature {
    pub fn new_from_file(filepath: &str) -> Result<Vec<Self>, &'static str> {
        let path = Path::new(filepath);
        // Parse JSON File into Value Struct
        let content = fs::read_to_string(Path::new(path)).unwrap();
        let val: Value = serde_json::from_str(&content).unwrap();
        let mut features = vec![];

        let mut dict: HashMap<String, u256> = HashMap::new();

        // Get all features from val
        let props = val["properties"].as_object().unwrap();
        for (key, value) in props {
            let name = key.to_string();
            let value = build_ast_root(value.as_str().unwrap())
                .unwrap()
                .evaluate()
                .unwrap();
            match value {
                ASTConstant::Number(n) => dict.insert(name, n),
                ASTConstant::SignedNumber(n) => dict.insert(name, n.as_u256()),
                _ => return Err("Not a number"),
            };
        }

        Ok(features)
    }
}

pub struct Model {
    pub(crate) current_model: Box<dyn AIModel>,
}

// pub fn get_current_model() -> &'static mut Model {
//     static mut MAYBE: MaybeUninit<Model> = MaybeUninit::uninit();
//     static ONLY: std::sync::Once = Once::new();

//     unsafe {
//         ONLY.call_once(|| {
//             MAYBE.write(Model {
//                 current_model: Box::new(IsolationForest::new()),
//             });
//         });
//         MAYBE.assume_init_mut()
//     }
// }

struct Dummy {}
impl AIModel for Dummy {
    fn train(&mut self) {}
    fn predict(&self, _sample: &[u256; FEATURE_VEC_LENGTH]) -> Option<f64> {
        None
    }
    fn add_samples(&mut self, _features: Vec<[u256; FEATURE_VEC_LENGTH]>) {}
    fn add_sample(&mut self, _features: [u256; FEATURE_VEC_LENGTH]) {}
}
