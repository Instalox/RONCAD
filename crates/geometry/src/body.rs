use roncad_core::ids::FeatureId;

#[derive(Debug, Clone)]
pub struct Body {
    pub name: String,
    pub features: Vec<FeatureId>,
    mesh_revision: u64,
}

impl Body {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            features: Vec::new(),
            mesh_revision: 0,
        }
    }

    pub fn push_feature(&mut self, feature: FeatureId) {
        self.features.push(feature);
        self.bump_mesh_revision();
    }

    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    pub fn mesh_revision(&self) -> u64 {
        self.mesh_revision
    }

    pub fn bump_mesh_revision(&mut self) {
        self.mesh_revision = self.mesh_revision.saturating_add(1);
    }
}
