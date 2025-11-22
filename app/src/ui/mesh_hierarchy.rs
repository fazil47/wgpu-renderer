use std::collections::{HashMap, HashSet};

use ecs::{Entity, World};

use crate::{
    rendering::WorldExtractExt,
    transform::{Name, Transform},
};

pub struct MeshHierarchy {
    pub roots: Vec<Entity>,
    pub children: HashMap<Entity, Vec<Entity>>,
    pub labels: HashMap<Entity, String>,
    pub renderables: HashSet<Entity>,
}

pub fn build_mesh_hierarchy(world: &World) -> MeshHierarchy {
    let renderables: HashSet<Entity> = world.get_renderables().into_iter().collect();
    let mut stack: Vec<Entity> = renderables.iter().copied().collect();
    let mut transforms = renderables.clone();

    // Walk up the parent chain so the tree includes ancestors of renderable entities.
    while let Some(entity) = stack.pop() {
        if let Some(transform) = world.get_component::<Transform>(entity)
            && let Some(parent) = transform.parent
            && transforms.insert(parent)
        {
            stack.push(parent);
        }
    }

    let mut labels = HashMap::new();
    for &entity in &transforms {
        labels.insert(entity, get_display_name(world, entity));
    }

    let mut children: HashMap<Entity, Vec<Entity>> = HashMap::new();
    let mut has_parent: HashSet<Entity> = HashSet::new();

    for &entity in &transforms {
        if let Some(transform) = world.get_component::<Transform>(entity)
            && let Some(parent) = transform.parent
            && transforms.contains(&parent)
        {
            children.entry(parent).or_default().push(entity);
            has_parent.insert(entity);
        }
    }

    for child_list in children.values_mut() {
        child_list.sort_by(|a, b| labels.get(a).unwrap().cmp(labels.get(b).unwrap()));
    }

    let mut roots: Vec<Entity> = transforms
        .iter()
        .copied()
        .filter(|entity| !has_parent.contains(entity))
        .collect();

    roots.sort_by(|a, b| labels.get(a).unwrap().cmp(labels.get(b).unwrap()));

    MeshHierarchy {
        roots,
        children,
        labels,
        renderables,
    }
}

pub fn draw_mesh_hierarchy(
    ui: &mut egui::Ui,
    hierarchy: &MeshHierarchy,
    selected: &mut Option<Entity>,
) {
    if hierarchy.roots.is_empty() {
        ui.label("No meshes available");
        return;
    }

    for &root in &hierarchy.roots {
        draw_mesh_node(ui, root, hierarchy, selected);
    }
}

fn draw_mesh_node(
    ui: &mut egui::Ui,
    entity: Entity,
    hierarchy: &MeshHierarchy,
    selected: &mut Option<Entity>,
) {
    let label = hierarchy
        .labels
        .get(&entity)
        .map(String::as_str)
        .unwrap_or("Entity");
    let children = hierarchy.children.get(&entity);
    let is_renderable = hierarchy.renderables.contains(&entity);
    let is_selected = selected.is_some_and(|e| e == entity);

    if let Some(children) = children {
        let header_label = if is_renderable && is_selected {
            egui::RichText::new(label).strong()
        } else {
            egui::RichText::new(label)
        };

        let response = egui::CollapsingHeader::new(header_label)
            .id_salt(entity.0)
            .default_open(true)
            .show(ui, |ui| {
                for &child in children {
                    draw_mesh_node(ui, child, hierarchy, selected);
                }
            });

        if is_renderable && response.header_response.clicked() {
            if is_selected {
                *selected = None;
            } else {
                selected.replace(entity);
            }
        }
    } else if is_renderable {
        let response = ui.selectable_label(is_selected, label);
        if response.clicked() {
            if is_selected {
                *selected = None;
            } else {
                selected.replace(entity);
            }
        }
    } else {
        ui.label(label);
    }
}

fn get_display_name(world: &World, entity: Entity) -> String {
    if let Some(name) = world.get_component::<Name>(entity) {
        let label = name.as_str();
        if !label.is_empty() {
            return label.to_owned();
        }
    }

    format!("Entity {}", entity.0)
}
