use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CounterGauge {
    metric: String,
    points: Vec<(u64, f64)>,
    tags: Vec<String>,
    common: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Metric {
    #[serde(rename = "gauge")]
    Gauge(CounterGauge),
    #[serde(rename = "gauge")]
    Counter(CounterGauge),
}
