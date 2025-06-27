use ethnum::{i256, u256, uint, AsU256};
use rand::Rng;
use std::collections::HashSet;

pub(crate) struct Sample {
    pub(crate) values: Vec<u256>,
}

pub(crate) struct IsolationForest {
    trees: Vec<IsolationTree>,
    sample_size: usize,
    max_depth: usize,
    trees_num: usize,
}

impl IsolationForest {
    fn new(sample_size: usize, max_depth: usize, trees_num: usize) -> Self {
        Self {
            trees: vec![],
            sample_size,
            max_depth,
            trees_num,
        }
    }

    /// Train the isolation forest
    fn fit(&mut self, samples: &[Sample]) {
        for _ in 0..self.trees_num {
            let mut rng = rand::thread_rng();
            let sample_indices: HashSet<usize> = (0..self.sample_size)
                .map(|_| rng.gen_range(0..samples.len()))
                .collect();
            let sample: Vec<&Sample> = sample_indices.iter().map(|&i| &samples[i]).collect();
            let tree = IsolationTree::fit(&sample, 0, self.max_depth);
            self.trees.push(tree);
        }
    }

    /// Return the Anomaly Score for a given sample
    /// The Anomaly Score is a value between 0 and 1
    /// 0 means the sample is an anomaly
    /// 1 means the sample is not an anomaly
    ///
    /// The higher the Anomaly Score, the more likely the sample is not an anomaly
    fn anomaly_score(&self, sample: &Sample) -> f64 {
        let mut scores = vec![];
        for tree in &self.trees {
            scores.push(tree.anomaly_score(sample, 0));
        }
        scores.iter().sum::<f64>() / self.trees.len() as f64
    }
}

#[derive(Debug)]
struct IsolationTree {
    root: Option<Box<Node>>,
}

impl IsolationTree {
    fn fit(data: &[&Sample], current_depth: usize, max_depth: usize) -> Self {
        if current_depth >= max_depth || data.len() <= 1 {
            return IsolationTree { root: None };
        }

        let mut rng = rand::thread_rng();
        let split_dimension = rng.gen_range(0..data[0].values.len());

        let mut values: Vec<u256> = data.iter().map(|u| u.values[split_dimension]).collect();
        values.sort_unstable();

        let split_index = values.len() / 2;
        let split_value = values[split_index];

        let (left, right): (Vec<&Sample>, Vec<&Sample>) = data
            .iter()
            .partition(|&&u| u.values[split_dimension] <= split_value);

        let left_child = IsolationTree::fit(&left, current_depth + 1, max_depth);
        let right_child = IsolationTree::fit(&right, current_depth + 1, max_depth);

        let node = Node {
            split_dimension,
            split_value,
            left: Some(Box::new(left_child)),
            right: Some(Box::new(right_child)),
        };

        IsolationTree {
            root: Some(Box::new(node)),
        }
    }

    /// Return the Anomaly Score for a given sample
    /// The Anomaly Score is a value between 0 and 1
    /// 0 means the sample is an anomaly
    /// 1 means the sample is not an anomaly
    ///
    /// The higher the Anomaly Score, the more likely the sample is not an anomaly
    fn anomaly_score(&self, sample: &Sample, current_depth: usize) -> f64 {
        match &self.root {
            Some(root) => self.anomaly_score_recursive(root, sample, current_depth),
            None => 0.0,
        }
    }

    fn anomaly_score_recursive(&self, node: &Node, sample: &Sample, current_depth: usize) -> f64 {
        let diff = (sample.values[node.split_dimension].as_i256() - node.split_value.as_i256())
            .abs()
            .as_f64();

        let scale = 1000000000.0;
        let scaled_diff = diff / scale;
        // println!("diff: {}", diff);
        // println!("scaled diff: {}", scaled_diff);
        // let mut score = 2.0_f64.powf(-scaled_diff / (1e-10 + f64::EPSILON));
        let training_set = (10 - 1) as f64;
        let mut score = 2.0_f64.powf(
            -scaled_diff
                / (2_f64 * (training_set.ln() + f64::EPSILON)
                    - (2_f64 * training_set / training_set + 1.0)),
        );
        // println!("score: {}", score);

        match &node.left {
            Some(left_child) => match &left_child.as_ref().root {
                Some(n) => {
                    if sample.values[node.split_dimension] <= node.split_value {
                        score *= self.anomaly_score_recursive(n.as_ref(), sample, current_depth + 1)
                    }
                }
                None => {}
            },
            None => {}
        }

        match &node.right {
            Some(right_child) => match &right_child.as_ref().root {
                Some(n) => {
                    if sample.values[node.split_dimension] > node.split_value {
                        score *= self.anomaly_score_recursive(n.as_ref(), sample, current_depth + 1)
                    }
                }
                None => {}
            },
            None => {}
        }
        score
    }

    // fn anomaly_score(&self, sample: &Sample, current_depth: usize) -> f64 {
    //     match &self.root {
    //         Some(root) => {
    //             let mut score = 1.0;
    //             let mut node = Some(root.as_ref());
    //             let mut depth = current_depth;

    //             while let Some(n) = node {
    //                 let diff = (sample.values[n.split_dimension].cmp(&n.split_value) as i8) as f64;
    //                 score *= 2.0_f64.powf(-diff.abs() / (1e-10 + f64::EPSILON));

    //                 if let Some(left_node) = &n.left {
    //                     if let Some(left_child) = &left_node.root {
    //                         if sample.values[n.split_dimension] <= n.split_value {
    //                             node = Some(left_child.as_ref());
    //                         }
    //                     }
    //                 }

    //                 if let Some(right_node) = &n.right {
    //                     if let Some(right_child) = &right_node.root {
    //                         if sample.values[n.split_dimension] > n.split_value {
    //                             node = Some(right_child.as_ref());
    //                         }
    //                     }
    //                 }

    //                 depth += 1;
    //             }
    //             let c = if current_depth > 0 {
    //                 2.0_f64.powf(-(current_depth as f64))
    //             } else {
    //                 1.0
    //             };
    //             score * c
    //         }
    //         None => 0.0,
    //     }
    // }
}

#[derive(Debug)]
struct Node {
    split_dimension: usize,
    split_value: u256,
    left: Option<Box<IsolationTree>>,
    right: Option<Box<IsolationTree>>,
}

#[test]
fn test_isolation_forest() {
    // Sample data
    let data = vec![
        Sample {
            values: vec![
                uint!("1000000"),
                uint!("2000000"),
                uint!("3000000"),
                uint!("4000000"),
                uint!("5000000"),
                uint!("6000000"),
                uint!("7000000"),
                uint!("8000000"),
                uint!("9000000"),
                uint!("10000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("40000000"),
                uint!("30000000"),
                uint!("20000000"),
                uint!("10000000"),
                uint!("50000000"),
                uint!("60000000"),
                uint!("70000000"),
                uint!("80000000"),
                uint!("90000000"),
                uint!("40000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("50000000"),
                uint!("60000000"),
                uint!("70000000"),
                uint!("80000000"),
                uint!("90000000"),
                uint!("100000000"),
                uint!("110000000"),
                uint!("120000000"),
                uint!("130000000"),
                uint!("50000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("60000000"),
                uint!("70000000"),
                uint!("80000000"),
                uint!("90000000"),
                uint!("100000000"),
                uint!("110000000"),
                uint!("120000000"),
                uint!("130000000"),
                uint!("140000000"),
                uint!("60000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("70000000"),
                uint!("80000000"),
                uint!("90000000"),
                uint!("100000000"),
                uint!("110000000"),
                uint!("120000000"),
                uint!("130000000"),
                uint!("140000000"),
                uint!("150000000"),
                uint!("70000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("56000000"),
                uint!("66000000"),
                uint!("76000000"),
                uint!("86000000"),
                uint!("96000000"),
                uint!("106000000"),
                uint!("116000000"),
                uint!("126000000"),
                uint!("136000000"),
                uint!("56000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("120000000"),
                uint!("130000000"),
                uint!("140000000"),
                uint!("150000000"),
                uint!("160000000"),
                uint!("170000000"),
                uint!("180000000"),
                uint!("190000000"),
                uint!("200000000"),
                uint!("120000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("230000000"),
                uint!("240000000"),
                uint!("250000000"),
                uint!("260000000"),
                uint!("270000000"),
                uint!("280000000"),
                uint!("290000000"),
                uint!("300000000"),
                uint!("310000000"),
                uint!("230000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("80000000"),
                uint!("70000000"),
                uint!("60000000"),
                uint!("50000000"),
                uint!("40000000"),
                uint!("30000000"),
                uint!("20000000"),
                uint!("10000000"),
                uint!("5000000"),
                uint!("80000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("90000000"),
                uint!("100000000"),
                uint!("110000000"),
                uint!("120000000"),
                uint!("130000000"),
                uint!("140000000"),
                uint!("150000000"),
                uint!("160000000"),
                uint!("170000000"),
                uint!("90000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("140000000"),
                uint!("150000000"),
                uint!("160000000"),
                uint!("170000000"),
                uint!("180000000"),
                uint!("190000000"),
                uint!("200000000"),
                uint!("210000000"),
                uint!("220000000"),
                uint!("140000000"),
            ],
        },
        Sample {
            values: vec![
                uint!("250000000"),
                uint!("260000000"),
                uint!("270000000"),
                uint!("280000000"),
                uint!("290000000"),
                uint!("300000000"),
                uint!("310000000"),
                uint!("320000000"),
                uint!("330000000"),
                uint!("250000000"),
            ],
        },
    ];

    let mut forest = IsolationForest::new(data.len(), 20, 100);
    forest.fit(&data);
    println!("Fit forest");

    // Test points
    let test_point_1 = Sample {
        values: vec![
            uint!("250000000"),
            uint!("251432434"),
            uint!("1"),
            uint!("560000000"),
            uint!("4200065000"),
            uint!("870011100"),
            uint!("31023440000"),
            uint!("1800006576800"),
            uint!("9"),
            uint!("6502341242150"),
        ],
    };
    let test_point_2 = Sample {
        values: vec![
            uint!("250000000"),
            uint!("260000000"),
            uint!("270000000"),
            uint!("280000000"),
            uint!("290000000"),
            uint!("300000000"),
            uint!("310000000"),
            uint!("320000000"),
            uint!("330000000"),
            uint!("250000000"),
        ],
    };

    // Calculate anomaly scores
    let score_1 = forest.anomaly_score(&test_point_1);
    let score_2 = forest.anomaly_score(&test_point_2);

    // Assert
    assert!(score_1 >= 0.0 && score_1 <= 1.0);
    assert!(score_2 >= 0.0 && score_2 <= 1.0);

    println!("Score 1: {}", score_1);
    println!("Score 2: {}", score_2);
}
