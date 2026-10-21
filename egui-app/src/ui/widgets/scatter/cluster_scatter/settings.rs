/// Feature space used for clustering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ClusterSpace {
    Objective,
    Variable,
    Combined,
}

impl ClusterSpace {
    pub fn label(&self) -> &'static str {
        match self {
            ClusterSpace::Objective => "Objective Space",
            ClusterSpace::Variable => "Variable Space",
            ClusterSpace::Combined => "Combined",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KSelectionMode {
    ElbowDefault,
    Manual,
}

impl KSelectionMode {
    pub fn label(&self) -> &'static str {
        match self {
            KSelectionMode::ElbowDefault => "Elbow (Auto)",
            KSelectionMode::Manual => "Manual",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum KMeansInitStrategy {
    KMeansPlusPlus,
    Deterministic,
}

impl KMeansInitStrategy {
    pub fn label(&self) -> &'static str {
        match self {
            KMeansInitStrategy::KMeansPlusPlus => "k-means++",
            KMeansInitStrategy::Deterministic => "Deterministic",
        }
    }
}

impl From<KMeansInitStrategy> for tunny_core::clustering::InitStrategy {
    fn from(s: KMeansInitStrategy) -> Self {
        match s {
            KMeansInitStrategy::KMeansPlusPlus => Self::KMeansPlusPlus,
            KMeansInitStrategy::Deterministic => Self::Deterministic,
        }
    }
}
