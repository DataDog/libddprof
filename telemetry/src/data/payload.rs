use crate::data::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "request_type", content = "payload")]
pub enum Payload {
    #[serde(rename = "app-started")]
    AppStarted(AppStarted),
    #[serde(rename = "app-dependencies-loaded")]
    AppDependenciesLoaded(AppDependenciesLoaded),
    #[serde(rename = "app-integrations-change")]
    AppIntegrationsChange(AppIntegrationsChange),
    #[serde(rename = "app-heartbeat")]
    AppHearbeat(()),
    #[serde(rename = "app-closing")]
    AppClosing(()),
    #[serde(rename = "generate-metrics")]
    GenerateMetrics(GenerateMetrics),
    #[serde(rename = "logs")]
    Logs(Vec<Log>),
}
