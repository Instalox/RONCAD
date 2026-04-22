//! On-disk schema for project files. Versioned so migration can slot in later.

use std::collections::HashMap;

use glam::{DVec2, DVec3};
use roncad_core::constraint::EntityPoint;
use roncad_core::ids::{
    BodyId, ConstraintId, FeatureId, SketchDimensionId, SketchEntityId, SketchId, WorkplaneId,
};
use roncad_geometry::feature::{ExtrudeFeature, RevolveFeature};
use roncad_geometry::{
    Body, Constraint, Feature, Project, Sketch, SketchDimension, SketchEntity, SketchProfile,
    Workplane,
};
use serde::{Deserialize, Serialize};
use slotmap::SlotMap;

use crate::error::{ProjectIoError, Result};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

type FileId = u64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectFile {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub active_sketch: Option<FileId>,
    #[serde(default)]
    pub workplanes: Vec<WorkplaneFile>,
    #[serde(default)]
    pub sketches: Vec<SketchFile>,
    #[serde(default)]
    pub bodies: Vec<BodyFile>,
    #[serde(default)]
    pub features: Vec<FeatureFile>,
}

impl Default for ProjectFile {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: "Untitled".to_string(),
            active_sketch: None,
            workplanes: Vec::new(),
            sketches: Vec::new(),
            bodies: Vec::new(),
            features: Vec::new(),
        }
    }
}

impl ProjectFile {
    pub fn from_project(project: &Project) -> Result<Self> {
        let workplane_ids = file_ids(project.workplanes.keys());
        let sketch_ids = file_ids(project.sketches.keys());
        let body_ids = file_ids(project.bodies.keys());
        let feature_ids = file_ids(project.features.keys());

        let workplanes = project
            .workplanes
            .iter()
            .map(|(id, workplane)| {
                Ok(WorkplaneFile {
                    id: lookup_file_id(&workplane_ids, id, "workplane")?,
                    name: workplane.name.clone(),
                    origin: workplane.origin,
                    u: workplane.u,
                    v: workplane.v,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let sketches = project
            .sketches
            .iter()
            .map(|(id, sketch)| {
                sketch_to_file(
                    lookup_file_id(&sketch_ids, id, "sketch")?,
                    sketch,
                    &workplane_ids,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let bodies = project
            .bodies
            .iter()
            .map(|(id, body)| {
                body_to_file(lookup_file_id(&body_ids, id, "body")?, body, &feature_ids)
            })
            .collect::<Result<Vec<_>>>()?;

        let features = project
            .features
            .iter()
            .map(|(id, feature)| {
                feature_to_file(
                    lookup_file_id(&feature_ids, id, "feature")?,
                    feature,
                    &body_ids,
                    &sketch_ids,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        let active_sketch = project
            .active_sketch
            .map(|id| lookup_file_id(&sketch_ids, id, "active sketch"))
            .transpose()?;

        Ok(Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: project.name.clone(),
            active_sketch,
            workplanes,
            sketches,
            bodies,
            features,
        })
    }

    pub fn into_project(self) -> Result<Project> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ProjectIoError::unsupported_schema(self.schema_version));
        }

        let mut workplanes = SlotMap::<WorkplaneId, Workplane>::with_key();
        let mut workplane_ids = HashMap::<FileId, WorkplaneId>::new();
        for workplane in self.workplanes {
            insert_file_id(
                &mut workplane_ids,
                workplane.id,
                workplanes.insert(Workplane {
                    name: workplane.name,
                    origin: workplane.origin,
                    u: workplane.u,
                    v: workplane.v,
                }),
                "workplane",
            )?;
        }

        let mut sketches = SlotMap::<SketchId, Sketch>::with_key();
        let mut sketch_ids = HashMap::<FileId, SketchId>::new();
        for sketch in self.sketches {
            let file_id = sketch.id;
            let sketch = sketch.into_sketch(&workplane_ids)?;
            let sketch_id = sketches.insert(sketch);
            insert_file_id(&mut sketch_ids, file_id, sketch_id, "sketch")?;
        }

        let mut bodies = SlotMap::<BodyId, Body>::with_key();
        let mut body_ids = HashMap::<FileId, BodyId>::new();
        let mut pending_body_features = Vec::<(BodyId, Vec<FileId>)>::new();
        for body in self.bodies {
            let body_id = bodies.insert(Body::new(body.name));
            insert_file_id(&mut body_ids, body.id, body_id, "body")?;
            pending_body_features.push((body_id, body.features));
        }

        let mut features = SlotMap::<FeatureId, Feature>::with_key();
        let mut feature_ids = HashMap::<FileId, FeatureId>::new();
        for feature in self.features {
            let file_id = feature.id;
            let feature = feature.into_feature(&body_ids, &sketch_ids)?;
            let feature_id = features.insert(feature);
            insert_file_id(&mut feature_ids, file_id, feature_id, "feature")?;
        }

        for (body_id, feature_refs) in pending_body_features {
            let body = bodies
                .get_mut(body_id)
                .ok_or(ProjectIoError::DanglingReference { kind: "body" })?;
            for feature_ref in feature_refs {
                body.push_feature(lookup_slot_id(&feature_ids, feature_ref, "feature")?);
            }
        }

        let active_sketch = self
            .active_sketch
            .map(|id| lookup_slot_id(&sketch_ids, id, "active sketch"))
            .transpose()?;

        Ok(Project::from_parts(
            self.name,
            workplanes,
            sketches,
            bodies,
            features,
            active_sketch,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkplaneFile {
    pub id: FileId,
    pub name: String,
    pub origin: DVec3,
    pub u: DVec3,
    pub v: DVec3,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SketchFile {
    pub id: FileId,
    pub name: String,
    pub workplane: FileId,
    #[serde(default)]
    pub entities: Vec<SketchEntityFileRecord>,
    #[serde(default)]
    pub dimensions: Vec<SketchDimensionFileRecord>,
    #[serde(default)]
    pub constraints: Vec<ConstraintFileRecord>,
}

impl SketchFile {
    fn into_sketch(self, workplane_ids: &HashMap<FileId, WorkplaneId>) -> Result<Sketch> {
        let workplane = lookup_slot_id(workplane_ids, self.workplane, "workplane")?;
        let mut sketch = Sketch::new(self.name, workplane);
        let mut entity_ids = HashMap::<FileId, SketchEntityId>::new();

        for entity in self.entities {
            let entity_id = sketch.add(entity.entity.into());
            insert_file_id(&mut entity_ids, entity.id, entity_id, "sketch entity")?;
        }

        let mut dimension_ids = HashMap::<FileId, SketchDimensionId>::new();
        for dimension in self.dimensions {
            let dimension_id = sketch.add_dimension(dimension.dimension.into());
            insert_file_id(
                &mut dimension_ids,
                dimension.id,
                dimension_id,
                "sketch dimension",
            )?;
        }

        let mut constraint_ids = HashMap::<FileId, ConstraintId>::new();
        for constraint in self.constraints {
            let constraint_id =
                sketch.add_constraint(constraint.constraint.into_constraint(&entity_ids)?);
            insert_file_id(
                &mut constraint_ids,
                constraint.id,
                constraint_id,
                "constraint",
            )?;
        }

        Ok(sketch)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SketchEntityFileRecord {
    pub id: FileId,
    #[serde(flatten)]
    pub entity: SketchEntityFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SketchEntityFile {
    Point {
        p: DVec2,
    },
    Line {
        a: DVec2,
        b: DVec2,
    },
    Rectangle {
        corner_a: DVec2,
        corner_b: DVec2,
    },
    Circle {
        center: DVec2,
        radius: f64,
    },
    Arc {
        center: DVec2,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    },
}

impl From<&SketchEntity> for SketchEntityFile {
    fn from(entity: &SketchEntity) -> Self {
        match entity {
            SketchEntity::Point { p } => Self::Point { p: *p },
            SketchEntity::Line { a, b } => Self::Line { a: *a, b: *b },
            SketchEntity::Rectangle { corner_a, corner_b } => Self::Rectangle {
                corner_a: *corner_a,
                corner_b: *corner_b,
            },
            SketchEntity::Circle { center, radius } => Self::Circle {
                center: *center,
                radius: *radius,
            },
            SketchEntity::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            } => Self::Arc {
                center: *center,
                radius: *radius,
                start_angle: *start_angle,
                sweep_angle: *sweep_angle,
            },
        }
    }
}

impl From<SketchEntityFile> for SketchEntity {
    fn from(entity: SketchEntityFile) -> Self {
        match entity {
            SketchEntityFile::Point { p } => Self::Point { p },
            SketchEntityFile::Line { a, b } => Self::Line { a, b },
            SketchEntityFile::Rectangle { corner_a, corner_b } => {
                Self::Rectangle { corner_a, corner_b }
            }
            SketchEntityFile::Circle { center, radius } => Self::Circle { center, radius },
            SketchEntityFile::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            } => Self::Arc {
                center,
                radius,
                start_angle,
                sweep_angle,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SketchDimensionFileRecord {
    pub id: FileId,
    #[serde(flatten)]
    pub dimension: SketchDimensionFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SketchDimensionFile {
    Distance { start: DVec2, end: DVec2 },
}

impl From<&SketchDimension> for SketchDimensionFile {
    fn from(dimension: &SketchDimension) -> Self {
        match dimension {
            SketchDimension::Distance { start, end } => Self::Distance {
                start: *start,
                end: *end,
            },
        }
    }
}

impl From<SketchDimensionFile> for SketchDimension {
    fn from(dimension: SketchDimensionFile) -> Self {
        match dimension {
            SketchDimensionFile::Distance { start, end } => Self::Distance { start, end },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstraintFileRecord {
    pub id: FileId,
    #[serde(flatten)]
    pub constraint: ConstraintFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConstraintFile {
    Coincident {
        a: EntityPointFile,
        b: EntityPointFile,
    },
    PointOnEntity {
        point: EntityPointFile,
        entity: FileId,
    },
    Horizontal {
        entity: FileId,
    },
    Vertical {
        entity: FileId,
    },
    Parallel {
        a: FileId,
        b: FileId,
    },
    Perpendicular {
        a: FileId,
        b: FileId,
    },
    Tangent {
        line: FileId,
        curve: FileId,
    },
    EqualLength {
        a: FileId,
        b: FileId,
    },
    EqualRadius {
        a: FileId,
        b: FileId,
    },
}

impl ConstraintFile {
    fn from_constraint(
        constraint: &Constraint,
        entity_ids: &HashMap<SketchEntityId, FileId>,
    ) -> Result<Self> {
        Ok(match constraint {
            Constraint::Coincident { a, b } => Self::Coincident {
                a: EntityPointFile::from_entity_point(*a, entity_ids)?,
                b: EntityPointFile::from_entity_point(*b, entity_ids)?,
            },
            Constraint::PointOnEntity { point, entity } => Self::PointOnEntity {
                point: EntityPointFile::from_entity_point(*point, entity_ids)?,
                entity: lookup_file_id(entity_ids, *entity, "sketch entity")?,
            },
            Constraint::Horizontal { entity } => Self::Horizontal {
                entity: lookup_file_id(entity_ids, *entity, "sketch entity")?,
            },
            Constraint::Vertical { entity } => Self::Vertical {
                entity: lookup_file_id(entity_ids, *entity, "sketch entity")?,
            },
            Constraint::Parallel { a, b } => Self::Parallel {
                a: lookup_file_id(entity_ids, *a, "sketch entity")?,
                b: lookup_file_id(entity_ids, *b, "sketch entity")?,
            },
            Constraint::Perpendicular { a, b } => Self::Perpendicular {
                a: lookup_file_id(entity_ids, *a, "sketch entity")?,
                b: lookup_file_id(entity_ids, *b, "sketch entity")?,
            },
            Constraint::Tangent { line, curve } => Self::Tangent {
                line: lookup_file_id(entity_ids, *line, "sketch entity")?,
                curve: lookup_file_id(entity_ids, *curve, "sketch entity")?,
            },
            Constraint::EqualLength { a, b } => Self::EqualLength {
                a: lookup_file_id(entity_ids, *a, "sketch entity")?,
                b: lookup_file_id(entity_ids, *b, "sketch entity")?,
            },
            Constraint::EqualRadius { a, b } => Self::EqualRadius {
                a: lookup_file_id(entity_ids, *a, "sketch entity")?,
                b: lookup_file_id(entity_ids, *b, "sketch entity")?,
            },
        })
    }

    fn into_constraint(self, entity_ids: &HashMap<FileId, SketchEntityId>) -> Result<Constraint> {
        Ok(match self {
            Self::Coincident { a, b } => Constraint::Coincident {
                a: a.into_entity_point(entity_ids)?,
                b: b.into_entity_point(entity_ids)?,
            },
            Self::PointOnEntity { point, entity } => Constraint::PointOnEntity {
                point: point.into_entity_point(entity_ids)?,
                entity: lookup_slot_id(entity_ids, entity, "sketch entity")?,
            },
            Self::Horizontal { entity } => Constraint::Horizontal {
                entity: lookup_slot_id(entity_ids, entity, "sketch entity")?,
            },
            Self::Vertical { entity } => Constraint::Vertical {
                entity: lookup_slot_id(entity_ids, entity, "sketch entity")?,
            },
            Self::Parallel { a, b } => Constraint::Parallel {
                a: lookup_slot_id(entity_ids, a, "sketch entity")?,
                b: lookup_slot_id(entity_ids, b, "sketch entity")?,
            },
            Self::Perpendicular { a, b } => Constraint::Perpendicular {
                a: lookup_slot_id(entity_ids, a, "sketch entity")?,
                b: lookup_slot_id(entity_ids, b, "sketch entity")?,
            },
            Self::Tangent { line, curve } => Constraint::Tangent {
                line: lookup_slot_id(entity_ids, line, "sketch entity")?,
                curve: lookup_slot_id(entity_ids, curve, "sketch entity")?,
            },
            Self::EqualLength { a, b } => Constraint::EqualLength {
                a: lookup_slot_id(entity_ids, a, "sketch entity")?,
                b: lookup_slot_id(entity_ids, b, "sketch entity")?,
            },
            Self::EqualRadius { a, b } => Constraint::EqualRadius {
                a: lookup_slot_id(entity_ids, a, "sketch entity")?,
                b: lookup_slot_id(entity_ids, b, "sketch entity")?,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "point", content = "entity", rename_all = "snake_case")]
pub enum EntityPointFile {
    Start(FileId),
    End(FileId),
    Center(FileId),
}

impl EntityPointFile {
    fn from_entity_point(
        point: EntityPoint,
        entity_ids: &HashMap<SketchEntityId, FileId>,
    ) -> Result<Self> {
        Ok(match point {
            EntityPoint::Start(id) => Self::Start(lookup_file_id(entity_ids, id, "sketch entity")?),
            EntityPoint::End(id) => Self::End(lookup_file_id(entity_ids, id, "sketch entity")?),
            EntityPoint::Center(id) => {
                Self::Center(lookup_file_id(entity_ids, id, "sketch entity")?)
            }
        })
    }

    fn into_entity_point(
        self,
        entity_ids: &HashMap<FileId, SketchEntityId>,
    ) -> Result<EntityPoint> {
        Ok(match self {
            Self::Start(id) => EntityPoint::Start(lookup_slot_id(entity_ids, id, "sketch entity")?),
            Self::End(id) => EntityPoint::End(lookup_slot_id(entity_ids, id, "sketch entity")?),
            Self::Center(id) => {
                EntityPoint::Center(lookup_slot_id(entity_ids, id, "sketch entity")?)
            }
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BodyFile {
    pub id: FileId,
    pub name: String,
    #[serde(default)]
    pub features: Vec<FileId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureFile {
    pub id: FileId,
    #[serde(flatten)]
    pub feature: FeatureFileKind,
}

impl FeatureFile {
    fn into_feature(
        self,
        body_ids: &HashMap<FileId, BodyId>,
        sketch_ids: &HashMap<FileId, SketchId>,
    ) -> Result<Feature> {
        self.feature.into_feature(body_ids, sketch_ids)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FeatureFileKind {
    Extrude {
        name: String,
        body: FileId,
        source_sketch: Option<FileId>,
        profile: SketchProfileFile,
        distance_mm: f64,
    },
    Revolve {
        name: String,
        body: FileId,
        source_sketch: Option<FileId>,
        profile: SketchProfileFile,
        axis_origin: DVec2,
        axis_dir: DVec2,
        angle_rad: f64,
    },
}

impl FeatureFileKind {
    fn into_feature(
        self,
        body_ids: &HashMap<FileId, BodyId>,
        sketch_ids: &HashMap<FileId, SketchId>,
    ) -> Result<Feature> {
        Ok(match self {
            Self::Extrude {
                name,
                body,
                source_sketch,
                profile,
                distance_mm,
            } => Feature::Extrude(ExtrudeFeature::new(
                name,
                lookup_slot_id(body_ids, body, "body")?,
                source_sketch
                    .map(|id| lookup_slot_id(sketch_ids, id, "source sketch"))
                    .transpose()?,
                profile.into(),
                distance_mm,
            )),
            Self::Revolve {
                name,
                body,
                source_sketch,
                profile,
                axis_origin,
                axis_dir,
                angle_rad,
            } => Feature::Revolve(RevolveFeature::new(
                name,
                lookup_slot_id(body_ids, body, "body")?,
                source_sketch
                    .map(|id| lookup_slot_id(sketch_ids, id, "source sketch"))
                    .transpose()?,
                profile.into(),
                axis_origin,
                axis_dir,
                angle_rad,
            )),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SketchProfileFile {
    Polygon { points: Vec<DVec2> },
    Circle { center: DVec2, radius: f64 },
}

impl From<&SketchProfile> for SketchProfileFile {
    fn from(profile: &SketchProfile) -> Self {
        match profile {
            SketchProfile::Polygon { points } => Self::Polygon {
                points: points.clone(),
            },
            SketchProfile::Circle { center, radius } => Self::Circle {
                center: *center,
                radius: *radius,
            },
        }
    }
}

impl From<SketchProfileFile> for SketchProfile {
    fn from(profile: SketchProfileFile) -> Self {
        match profile {
            SketchProfileFile::Polygon { points } => Self::Polygon { points },
            SketchProfileFile::Circle { center, radius } => Self::Circle { center, radius },
        }
    }
}

fn sketch_to_file(
    id: FileId,
    sketch: &Sketch,
    workplane_ids: &HashMap<WorkplaneId, FileId>,
) -> Result<SketchFile> {
    let entity_ids = file_ids(sketch.entities.keys());

    let entities = sketch
        .entities
        .iter()
        .map(|(id, entity)| {
            Ok(SketchEntityFileRecord {
                id: lookup_file_id(&entity_ids, id, "sketch entity")?,
                entity: entity.into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let dimension_ids = file_ids(sketch.dimensions.keys());
    let dimensions = sketch
        .dimensions
        .iter()
        .map(|(id, dimension)| {
            Ok(SketchDimensionFileRecord {
                id: lookup_file_id(&dimension_ids, id, "sketch dimension")?,
                dimension: dimension.into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let constraint_ids = file_ids(sketch.constraints.keys());
    let constraints = sketch
        .constraints
        .iter()
        .map(|(id, constraint)| {
            Ok(ConstraintFileRecord {
                id: lookup_file_id(&constraint_ids, id, "constraint")?,
                constraint: ConstraintFile::from_constraint(constraint, &entity_ids)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(SketchFile {
        id,
        name: sketch.name.clone(),
        workplane: lookup_file_id(workplane_ids, sketch.workplane, "workplane")?,
        entities,
        dimensions,
        constraints,
    })
}

fn body_to_file(
    id: FileId,
    body: &Body,
    feature_ids: &HashMap<FeatureId, FileId>,
) -> Result<BodyFile> {
    Ok(BodyFile {
        id,
        name: body.name.clone(),
        features: body
            .features
            .iter()
            .map(|id| lookup_file_id(feature_ids, *id, "feature"))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn feature_to_file(
    id: FileId,
    feature: &Feature,
    body_ids: &HashMap<BodyId, FileId>,
    sketch_ids: &HashMap<SketchId, FileId>,
) -> Result<FeatureFile> {
    let feature = match feature {
        Feature::Extrude(feature) => FeatureFileKind::Extrude {
            name: feature.name.clone(),
            body: lookup_file_id(body_ids, feature.body, "body")?,
            source_sketch: feature
                .source_sketch
                .map(|id| lookup_file_id(sketch_ids, id, "source sketch"))
                .transpose()?,
            profile: (&feature.profile).into(),
            distance_mm: feature.distance_mm,
        },
        Feature::Revolve(feature) => FeatureFileKind::Revolve {
            name: feature.name.clone(),
            body: lookup_file_id(body_ids, feature.body, "body")?,
            source_sketch: feature
                .source_sketch
                .map(|id| lookup_file_id(sketch_ids, id, "source sketch"))
                .transpose()?,
            profile: (&feature.profile).into(),
            axis_origin: feature.axis_origin,
            axis_dir: feature.axis_dir,
            angle_rad: feature.angle_rad,
        },
    };

    Ok(FeatureFile { id, feature })
}

fn file_ids<K: Copy + Eq + std::hash::Hash>(keys: impl Iterator<Item = K>) -> HashMap<K, FileId> {
    keys.enumerate()
        .map(|(index, key)| (key, index as FileId + 1))
        .collect()
}

fn lookup_file_id<K: Copy + Eq + std::hash::Hash>(
    ids: &HashMap<K, FileId>,
    key: K,
    kind: &'static str,
) -> Result<FileId> {
    ids.get(&key)
        .copied()
        .ok_or(ProjectIoError::DanglingReference { kind })
}

fn lookup_slot_id<K: Copy>(ids: &HashMap<FileId, K>, id: FileId, kind: &'static str) -> Result<K> {
    ids.get(&id)
        .copied()
        .ok_or(ProjectIoError::MissingReference { kind, id })
}

fn insert_file_id<K: Copy>(
    ids: &mut HashMap<FileId, K>,
    file_id: FileId,
    slot_id: K,
    kind: &'static str,
) -> Result<K> {
    if ids.insert(file_id, slot_id).is_some() {
        return Err(ProjectIoError::DuplicateId { kind, id: file_id });
    }
    Ok(slot_id)
}

#[cfg(test)]
mod tests {
    use glam::dvec2;

    use super::*;
    use crate::{project_from_json, project_to_json};

    #[test]
    fn default_file_still_matches_minimal_project_stub() {
        let file = ProjectFile::default();

        assert_eq!(file.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(file.name, "Untitled");
        assert!(file.workplanes.is_empty());
        assert!(file.sketches.is_empty());
    }

    #[test]
    fn round_trips_default_project() {
        let project = Project::new_untitled();

        let json = project_to_json(&project).expect("serialize project");
        let loaded = project_from_json(&json).expect("load project");

        assert_eq!(loaded.name, project.name);
        assert_eq!(loaded.workplanes.len(), 3);
        assert_eq!(loaded.sketches.len(), 1);
        assert!(loaded.active_sketch.is_some());
        assert_eq!(
            loaded.active_workplane().map(|plane| plane.name.as_str()),
            Some("XY")
        );
    }

    #[test]
    fn round_trips_constraints_dimensions_and_features() {
        let mut project = Project::new_untitled();
        let sketch_id = project.active_sketch.expect("active sketch");
        let sketch = project.sketches.get_mut(sketch_id).expect("sketch");
        let line_a = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 0.0),
            b: dvec2(5.0, 0.0),
        });
        let line_b = sketch.add(SketchEntity::Line {
            a: dvec2(0.0, 3.0),
            b: dvec2(5.0, 3.0),
        });
        let circle = sketch.add(SketchEntity::Circle {
            center: dvec2(10.0, 0.0),
            radius: 2.0,
        });
        sketch.add_dimension(SketchDimension::Distance {
            start: dvec2(0.0, 0.0),
            end: dvec2(5.0, 0.0),
        });
        sketch.add_constraint(Constraint::Horizontal { entity: line_a });
        sketch.add_constraint(Constraint::Parallel {
            a: line_a,
            b: line_b,
        });
        sketch.add_constraint(Constraint::PointOnEntity {
            point: EntityPoint::Center(circle),
            entity: line_a,
        });

        let (body, feature) = project
            .extrude_profile(
                sketch_id,
                SketchProfile::Circle {
                    center: dvec2(10.0, 0.0),
                    radius: 2.0,
                },
                4.0,
            )
            .expect("extrude");

        let json = project_to_json(&project).expect("serialize project");
        let loaded = project_from_json(&json).expect("load project");

        assert_eq!(loaded.sketches.len(), 1);
        let loaded_sketch = loaded.active_sketch().expect("loaded sketch");
        assert_eq!(loaded_sketch.entities.len(), 3);
        assert_eq!(loaded_sketch.dimensions.len(), 1);
        assert_eq!(loaded_sketch.constraints.len(), 3);
        assert_eq!(loaded.bodies.len(), 1);
        assert_eq!(loaded.features.len(), 1);

        let loaded_body = loaded.bodies.values().next().expect("body");
        assert_eq!(loaded_body.name, project.bodies[body].name);
        assert_eq!(loaded_body.feature_count(), 1);
        let loaded_feature = loaded.features.values().next().expect("feature");
        assert_eq!(loaded_feature.name(), project.features[feature].name());
        assert_eq!(loaded_feature.source_sketch(), loaded.active_sketch);
        assert_eq!(
            loaded.body_volume_mm3(loaded_feature.body()),
            project.body_volume_mm3(body)
        );
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let file = ProjectFile {
            schema_version: CURRENT_SCHEMA_VERSION + 1,
            ..ProjectFile::default()
        };

        let err = file.into_project().expect_err("unsupported schema");

        assert!(matches!(
            err,
            ProjectIoError::UnsupportedSchemaVersion { .. }
        ));
    }

    #[test]
    fn rejects_missing_constraint_entity() {
        let file = ProjectFile {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: "Broken".to_string(),
            active_sketch: Some(1),
            workplanes: vec![WorkplaneFile {
                id: 1,
                name: "XY".to_string(),
                origin: DVec3::ZERO,
                u: DVec3::X,
                v: DVec3::Y,
            }],
            sketches: vec![SketchFile {
                id: 1,
                name: "Sketch".to_string(),
                workplane: 1,
                entities: Vec::new(),
                dimensions: Vec::new(),
                constraints: vec![ConstraintFileRecord {
                    id: 1,
                    constraint: ConstraintFile::Horizontal { entity: 99 },
                }],
            }],
            bodies: Vec::new(),
            features: Vec::new(),
        };

        let err = file.into_project().expect_err("missing entity");

        assert!(matches!(
            err,
            ProjectIoError::MissingReference {
                kind: "sketch entity",
                id: 99
            }
        ));
    }
}
