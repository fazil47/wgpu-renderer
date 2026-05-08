
# Mesh Hierarchy Calculation

The mesh hierarchy looks like this in the ECS:

![[MeshHierarchyCalculation1.excalidraw|1080]]

And I want the final data structure that the EGUI system consumes to be like this:

```rs
struct MeshHierarchyNode {
	entity: Entity,
	label: String,
	is_renderable: bool, // Entities with both Transform and Mesh components
	children: Vec<MeshHierarchyNode>
}

struct MeshHierarchy {
	root_nodes: Vec<MeshHierarchyNode>
}
```

The algorithm to turn generate `MeshHierarchy` from the ECS:

```
	entities = get entities with the Transform component from World
	
	nodes = []
	root_nodes = []
	entity_to_node = {} # map from entity to MeshHierarchyNode
	
	for entity in entities:
		is_renderable = false

		label = `entity::{entity}`
		
		if entity has a Mesh component:
			is_renderable = true
			
		if entity has a Name component:
			label = world.get_component<Name>(entity).name
		
		node = MeshHierarchyNode { entity, label, is_renderable, children = [] }
		nodes.push(node)
		
		transform = world.get_component<Transform>(entity)
		if !transform.parent:
			root_nodes.push(node)
		
		entity_to_node[entity] = node
		
	function populateChildren(node):
		if node.entity doesn't have a Children component:
			return

		node.children = world
			.get_component<Children>(node.entity)
			.entities
			.map(entity => {
				child_node = entity_to_node[entity]
				populateChildren(child_node)
				return child_node
			})
	
	for node in root_nodes:		
		populateChildren(node)
```