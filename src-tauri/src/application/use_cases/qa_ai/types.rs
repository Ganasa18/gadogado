use serde::Deserialize;

#[derive(Debug, Deserialize, serde::Serialize)]
pub(crate) struct SummaryOutput {
    pub(crate) summary_text: String,
    pub(crate) entities: Option<Vec<String>>,
    pub(crate) risks: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub(crate) struct TestCaseOutput {
    #[serde(default)]
    pub(crate) negative_cases: Vec<TestCaseInput>,
    #[serde(default)]
    pub(crate) edge_cases: Vec<TestCaseInput>,
    #[serde(default)]
    pub(crate) exploratory_charters: Vec<TestCaseInput>,
    #[serde(default)]
    pub(crate) api_gap_checks: Vec<TestCaseInput>,
}

#[derive(Debug, Deserialize, serde::Serialize, Clone)]
pub(crate) struct TestCaseInput {
    pub(crate) title: String,
    pub(crate) steps: Vec<String>,
    pub(crate) expected: Option<String>,
    pub(crate) priority: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub(crate) struct ExploreOutput {
    #[serde(default)]
    pub(crate) positive_case: Option<TestCaseInput>,
    #[serde(default)]
    pub(crate) negative_cases: Vec<TestCaseInput>,
    #[serde(default)]
    pub(crate) edge_cases: Vec<TestCaseInput>,
    #[serde(default)]
    pub(crate) exploratory_charters: Vec<TestCaseInput>,
}
