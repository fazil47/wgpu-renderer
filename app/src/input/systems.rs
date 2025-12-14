use ecs::World;
use maths::{Quat, Vec3};

use crate::{camera::Camera, input::CameraController, time::Time};

pub fn camera_controller_system(world: &mut World) {
    let dt = world.get_resource::<Time>().unwrap().delta_time;

    let mut controller = world.get_resource_mut::<CameraController>().unwrap();

    let camera_entity = world
        .get_entities_with::<Camera>()
        .first()
        .copied()
        .expect("Expected camera entity");

    if let Some(mut camera) = world.get_component_mut::<Camera>(camera_entity) {
        // Move the camera based on input
        let (forward, right, up) = {
            let forward = camera.forward.normalized();
            let up = camera.up.normalized();
            let right = forward.cross(up).normalized();
            (forward, right, up)
        };

        let mut velocity = Vec3::ZERO;
        velocity += forward * (controller.amount_forward - controller.amount_backward);
        velocity += right * (controller.amount_right - controller.amount_left);
        velocity += up * (controller.amount_up - controller.amount_down);

        if velocity.length() > 0.0 {
            let speed_multiplier = if controller.fast_speed_requests > 0 {
                controller.fast_speed_multiplier
            } else {
                1.0
            };
            camera.eye += velocity.normalized() * controller.speed * speed_multiplier * dt;
        }

        // Rotate the camera based on mouse input
        if controller.cursor_locked
            && (controller.rotate_horizontal != 0.0 || controller.rotate_vertical != 0.0)
        {
            // Horizontal rotation around world Y axis
            let yaw_rotation =
                Quat::from_rotation_y(-controller.rotate_horizontal * controller.sensitivity);

            // Vertical rotation around the camera's right axis
            let right_axis = camera.forward.cross(Vec3::Y).normalized();
            let pitch_rotation = Quat::from_axis_angle(
                right_axis,
                -controller.rotate_vertical * controller.sensitivity,
            );

            // Apply rotations to forward vector
            camera.forward = yaw_rotation * camera.forward;
            let new_forward = pitch_rotation * camera.forward;

            // Prevent camera from flipping upside down (clamp pitch)
            if new_forward.dot(Vec3::Y).abs() < 0.95 {
                camera.forward = new_forward.normalized();
            }

            // Update the camera's up vector to maintain orthogonality
            let right = camera.forward.cross(Vec3::Y).normalized();
            camera.up = right.cross(camera.forward).normalized();

            // Reset rotation deltas
            controller.rotate_horizontal = 0.0;
            controller.rotate_vertical = 0.0;
        }
    }
}
