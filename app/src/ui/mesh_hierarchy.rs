use std::collections::{HashMap, HashSet};

use ecs::{Entity, World};

use crate::{
    hierarchy::{ChildOf, Children},
    rendering::WorldExtractExt,
    transform::Name,
};

pub struct MeshHierarchyNode {
    pub entity: Entity,
    pub label: String,
    pub is_renderable: bool,
    pub children: Vec<MeshHierarchyNode>,
}

pub struct MeshHierarchy {
    pub roots: Vec<MeshHierarchyNode>,
}

pub fn build_mesh_hierarchy(world: &World) -> MeshHierarchy {
    let renderables: HashSet<Entity> = world.get_renderables().into_iter().collect();
    let mut relevant: HashSet<Entity> = renderables.clone();

    // Walk up the parent chain so the tree includes ancestors of renderable entities.
    let mut stack: Vec<Entity> = renderables.iter().copied().collect();
    while let Some(entity) = stack.pop() {
        if let Some(child_of) = world.get_component::<ChildOf>(entity)
            && relevant.insert(child_of.0)
        {
            stack.push(child_of.0);
        }
    }

    let mut labels = HashMap::new();
    for &entity in &relevant {
        labels.insert(entity, get_display_name(world, entity));
    }

    // Roots are relevant entities with no parent (or whose parent isn't relevant)
    let mut roots: Vec<Entity> = relevant
        .iter()
        .copied()
        .filter(|&entity| {
            world
                .get_component::<ChildOf>(entity)
                .is_none_or(|c| !relevant.contains(&c.0))
        })
        .collect();

    roots.sort_by(|a, b| labels.get(a).unwrap().cmp(labels.get(b).unwrap()));

    MeshHierarchy {
        roots: roots
            .into_iter()
            .map(|entity| build_hierarchy_node(world, entity, &labels, &renderables, &relevant))
            .collect(),
    }
}

fn build_hierarchy_node(
    world: &World,
    entity: Entity,
    labels: &HashMap<Entity, String>,
    renderables: &HashSet<Entity>,
    relevant: &HashSet<Entity>,
) -> MeshHierarchyNode {
    let mut child_entities: Vec<Entity> = world
        .get_component::<Children>(entity)
        .map(|c| {
            c.0.iter()
                .copied()
                .filter(|e| relevant.contains(e))
                .collect()
        })
        .unwrap_or_default();

    child_entities.sort_by(|a, b| labels.get(a).unwrap().cmp(labels.get(b).unwrap()));

    let children = child_entities
        .into_iter()
        .map(|child| build_hierarchy_node(world, child, labels, renderables, relevant))
        .collect();

    MeshHierarchyNode {
        entity,
        label: labels
            .get(&entity)
            .cloned()
            .unwrap_or_else(|| "Entity".to_string()),
        is_renderable: renderables.contains(&entity),
        children,
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

    for root in &hierarchy.roots {
        draw_mesh_node(ui, root, selected);
    }
}

fn draw_mesh_node(ui: &mut egui::Ui, node: &MeshHierarchyNode, selected: &mut Option<Entity>) {
    let is_selected = selected.is_some_and(|entity| entity == node.entity);

    if !node.children.is_empty() {
        let header_label = if node.is_renderable && is_selected {
            egui::RichText::new(&node.label).strong()
        } else {
            egui::RichText::new(&node.label)
        };

        let response = egui::CollapsingHeader::new(header_label)
            .id_salt(node.entity.0)
            .default_open(true)
            .show(ui, |ui| {
                for child in &node.children {
                    draw_mesh_node(ui, child, selected);
                }
            });

        if node.is_renderable && response.header_response.clicked() {
            if is_selected {
                *selected = None;
            } else {
                selected.replace(node.entity);
            }
        }
    } else if node.is_renderable {
        let response = ui.selectable_label(is_selected, &node.label);
        if response.clicked() {
            if is_selected {
                *selected = None;
            } else {
                selected.replace(node.entity);
            }
        }
    } else {
        ui.label(&node.label);
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
