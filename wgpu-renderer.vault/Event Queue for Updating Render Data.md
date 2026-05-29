Currently if a mesh is moved, then the entire render data is torn down and rebuilt. Instead of doing that:
- Add events to the ECS
- When a transform has moved, add a `TransformMoved(entity)` event to the queue
	- Add an event handler that listens for that event and update the transform only for that entity

## Migration Plan

### Step 1
- Add an event system that supports events that target entities
	- Events are deduplicated based on entity and event type by using an `IndexSet`
- Remove `DirtyFlags.transform` in favor of a `TransformChanged` event
- In `update_system` just check if there are any `TransformChanged` event instead of `DirtyFlags.transform`

### Step 2

- `TransformChanged(entity)` should only be used to update the CPU and GPU transform data for the target entity