use crate::error::EditError;
use crate::layer::{BlendMode, LayerKind};
use crate::project::Project;
use crate::seed::{LayerId, Seed};
use crate::settings::{GalaxyBandSettings, NebulaSettings, StarFieldSettings, SunSettings};

const MAX_UNDO_STEPS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragField {
    Opacity,
    StarField,
    Nebula,
    GalaxyBand,
    Sun,
}

pub struct History {
    project: Project,
    undo_stack: Vec<Project>,
    redo_stack: Vec<Project>,
    last_drag: Option<(LayerId, DragField)>,
}

impl History {
    pub fn new(project: Project) -> History {
        History {
            project,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_drag: None,
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn add_layer(&mut self, kind: LayerKind) -> LayerId {
        self.edit(None, |project| Ok(project.add_layer(kind)))
            .expect("adding a layer cannot fail")
    }

    pub fn set_opacity(&mut self, layer: LayerId, opacity: f32) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::Opacity)), |project| {
            project.set_opacity(layer, opacity)
        })
    }

    pub fn set_blend(&mut self, layer: LayerId, blend: BlendMode) -> Result<(), EditError> {
        self.edit(None, |project| project.set_blend(layer, blend))
    }

    pub fn set_visible(&mut self, layer: LayerId, visible: bool) -> Result<(), EditError> {
        self.edit(None, |project| project.set_visible(layer, visible))
    }

    pub fn set_star_field_settings(
        &mut self,
        layer: LayerId,
        settings: StarFieldSettings,
    ) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::StarField)), |project| {
            project.set_star_field_settings(layer, settings)
        })
    }

    pub fn set_nebula_settings(
        &mut self,
        layer: LayerId,
        settings: NebulaSettings,
    ) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::Nebula)), |project| {
            project.set_nebula_settings(layer, settings)
        })
    }

    pub fn set_sun_settings(
        &mut self,
        layer: LayerId,
        settings: SunSettings,
    ) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::Sun)), |project| {
            project.set_sun_settings(layer, settings)
        })
    }

    pub fn set_galaxy_band_settings(
        &mut self,
        layer: LayerId,
        settings: GalaxyBandSettings,
    ) -> Result<(), EditError> {
        self.edit(Some((layer, DragField::GalaxyBand)), |project| {
            project.set_galaxy_band_settings(layer, settings)
        })
    }

    /// Sets the Seed and gives every Layer new random colors, as one undo step.
    pub fn randomize(&mut self, seed: Seed) {
        self.edit(None, |project| {
            project.set_seed(seed);
            project.randomize_colors(seed);
            Ok(())
        })
        .expect("randomizing cannot fail");
    }

    /// Gives one Layer a new look (see `Project::reroll_layer`), as one undo step.
    pub fn reroll_layer(&mut self, layer: LayerId, seed: Seed) -> Result<(), EditError> {
        self.edit(None, |project| project.reroll_layer(layer, seed))
    }

    pub fn set_seed(&mut self, seed: Seed) {
        self.edit(None, |project| {
            project.set_seed(seed);
            Ok(())
        })
        .expect("setting the seed cannot fail");
    }

    pub fn remove_layer(&mut self, layer: LayerId) -> Result<(), EditError> {
        self.edit(None, |project| project.remove_layer(layer))
    }

    pub fn move_layer(&mut self, layer: LayerId, new_index: usize) -> Result<(), EditError> {
        self.edit(None, |project| project.move_layer(layer, new_index))
    }

    pub fn duplicate_layer(&mut self, layer: LayerId) -> Result<LayerId, EditError> {
        self.edit(None, |project| project.duplicate_layer(layer))
    }

    /// Runs `change` and, only if it succeeds, records the prior state as an undo step.
    /// An edit whose `drag_of` names the same Layer and field as the previous edit's joins that
    /// edit's undo step instead of starting a new one.
    fn edit<T>(
        &mut self,
        drag_of: Option<(LayerId, DragField)>,
        change: impl FnOnce(&mut Project) -> Result<T, EditError>,
    ) -> Result<T, EditError> {
        let before = self.project.clone();
        let result = change(&mut self.project)?;
        let continues_drag = drag_of.is_some() && drag_of == self.last_drag;
        if !continues_drag {
            self.undo_stack.push(before);
            if self.undo_stack.len() > MAX_UNDO_STEPS {
                self.undo_stack.remove(0);
            }
        }
        self.redo_stack.clear();
        self.last_drag = drag_of;
        Ok(result)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            let current = std::mem::replace(&mut self.project, previous);
            self.redo_stack.push(current);
            self.last_drag = None;
        }
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            let current = std::mem::replace(&mut self.project, next);
            self.undo_stack.push(current);
            self.last_drag = None;
        }
    }
}
