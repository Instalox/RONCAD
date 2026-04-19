use roncad_core::ids::FeatureId;

#[derive(Debug, Clone)]
pub struct Body {
    pub name: String,
    pub features: Vec<FeatureId>,
}

impl Body {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            features: Vec::new(),
        }
    }

    pub fn push_feature(&mut self, feature: FeatureId) {
        self.features.push(feature);
    }

    pub fn feature_count(&self) -> usize {
        self.features.len()
    }
}
