use ethnum::u256;
use extended_isolation_forest::{Forest, ForestFloat, ForestOptions};

use crate::FEATURE_VEC_LENGTH;

use super::{AIModel, ModelFeature};

pub struct IsolationForest {
    pub(crate) samples: Vec<[u256; FEATURE_VEC_LENGTH]>,
    pub(crate) model: Option<extended_isolation_forest::Forest<u256, 10>>,
    pub(crate) avg_score: Option<f64>,
    pub(crate) avg_normalized_score: Option<f64>,
}

impl IsolationForest {
    pub fn new() -> Self {
        Self {
            samples: vec![],
            model: None,
            avg_score: None,
            avg_normalized_score: None,
        }
    }
}

impl AIModel for IsolationForest {
    /// Train the model with the given samples
    fn train(&mut self) {
        let opt = ForestOptions {
            n_trees: 150,
            sample_size: 200,
            max_tree_depth: None,
            extension_level: 1,
        };
        self.model = Some(Forest::from_slice(&self.samples.as_slice(), &opt).unwrap());
    }

    /// Predict the score
    /// Negative Scores indicate an Outlier
    /// Positive Scores indicate an Inlier
    fn predict(&self, sample: &[u256; FEATURE_VEC_LENGTH]) -> Option<f64> {
        match self.model {
            Some(forest) => Some(forest.score(sample)),
            None => None,
        }
    }

    /// Add multiple samples to the model
    /// Each sample should have an associated topic, for instance, the function selector called.
    fn add_samples(&mut self, features: Vec<[u256; FEATURE_VEC_LENGTH]>) {
        for feature in features {
            self.add_sample(feature);
        }
    }

    /// Add one sample to the model
    fn add_sample(&mut self, feature: [u256; FEATURE_VEC_LENGTH]) {
        self.samples.push(feature);
    }
}

/// Make a new feature with a key and a value
pub fn new_feature(key: &str, value: u64) -> isolation_forest::isolation_forest::Feature {
    isolation_forest::isolation_forest::Feature::new(key, value)
}
