use std::sync::{Arc, RwLock};

use cgmath::{InnerSpace, Quaternion, Rotation, Rotation3, Vector3, Zero};

use crate::{
    LOGIC_PROFILER, game_objects::transform, physics::{ColliderSystem, LeafInHierachy, RigidBody}
};

use super::{
    transform::{Transform, TransformID, TransformSystem},
    Camera, Rotate, Translate, TransformTracker,
};
use legion::*;

const CAM_SPEED: f32 = 6.;

pub struct Inputs {
    pub movement: Vector3<f32>,
}

impl Default for Inputs {
    fn default() -> Self {
        Self {
            movement: Vector3::zero(),
        }
    }
}
impl Inputs {
    pub fn move_transform(&self, transform: &mut Transform, seconds_passed: f32) {
        let view = transform.get_local_transform();

        let mut final_move = self.movement;
        final_move.y = 0.;

        // if self.w == Pressed {
        //     movement.z -= 1.; // forward
        // } else if self.s == Pressed {
        //     movement.z += 1.; // backwards
        // }
        // if self.a == Pressed {
        //     movement.x -= 1.; // left
        // } else if self.d == Pressed {
        //     movement.x += 1.; // right
        // }

        final_move = view.rotation.rotate_vector(final_move);
        final_move.y = 0.;
        if final_move != Vector3::zero() {
            final_move = final_move.normalize();
        }

        final_move.y = self.movement.y;

        // apply movement
        transform.set_translation(view.translation + final_move * CAM_SPEED * seconds_passed);
    }
}

/// stores game data and handles logic updates
pub struct GameWorld {
    pub transforms: TransformSystem,
    pub colliders: ColliderSystem,
    pub world: World,
    pub camera: Camera,
    pub fixed_seconds: f32,
    pub last_delta_time: f32,
    pub inputs: Inputs,
}

impl GameWorld {
    pub fn new() -> Self {
        let mut transforms = TransformSystem::new();
        let colliders = ColliderSystem::new();
        let mut world = World::default();
        let camera = Camera::from_transform(transforms.next().unwrap());
        world.push((camera.transform,));

        // colliders.

        Self {
            transforms,
            colliders,
            world,
            camera,
            fixed_seconds: 0.,
            last_delta_time: 0.,
            inputs: Inputs::default(),
        }
    }

    /// update world logic with a time step
    ///
    /// # Order
    /// 1. Rigidbody movement
    /// 2. Collision resolution
    /// 3. Other logic
    pub fn update(&mut self, seconds_passed: f32) {
        self.last_delta_time = seconds_passed;
        self.fixed_seconds += seconds_passed;

        // let mut profiler = unsafe { LOGIC_PROFILER.lock().unwrap() };
        let logic_start = std::time::Instant::now();

        // physics update
        let mut query = <(&TransformID, &mut Arc<RwLock<RigidBody>>)>::query();
        for (transfrom, rigid_body) in query.iter_mut(&mut self.world) {
            rigid_body.write().unwrap().update(
                self.transforms.get_transform_mut(transfrom).unwrap(),
                seconds_passed,
            );
            // println!(
            //     "[RB] id: {:?}, model: {:?}",
            //     transfrom,
            //     self.transforms.get_global_model(transfrom)
            // );
        }

        // [Profiling] Physics
        let phys_time = logic_start.elapsed().as_micros() as u32;
        let coll_start = std::time::Instant::now();

        // update bounds
        let mut query = <(&TransformID, &mut LeafInHierachy)>::query();
        // println!("frame start");
        for (id, collider) in query.iter_mut(&mut self.world) {
            if let Some(transform) = self.transforms.get_transform(id) {
                if transform.phys_modified {
                    self.colliders.update(collider, &mut self.transforms);
                    self.transforms.reset_phys_modified(id);
                }
            }
        }

        let contact_resolver = self.colliders.get_contacts(&mut self.transforms);
        contact_resolver.resolve(&mut self.transforms, seconds_passed);
        // store old velocity
        let mut query = <&mut Arc<RwLock<RigidBody>>>::query();
        for rigid_body in query.iter_mut(&mut self.world) {
            rigid_body.write().unwrap().set_old_velocity();
        }

        // [Profiling] Colliders
        let coll_time = coll_start.elapsed().as_micros() as u32;
        let lerp_start = std::time::Instant::now();

        // update interpolation models
        let mut query = <&TransformID>::query();
        for transform_id in query.iter(&self.world) {
            // *last_model =
            //     InterpolateTransform(self.transforms.get_global_model(transform_id).unwrap());
            if self.transforms.store_last_model(transform_id).is_err() {
                println!("[Error] Failed to find transform of interpolated object");
            }
        }
        self.transforms.update_last_fixed();

        // [Profiling] Interpolation
        let lerp_time = lerp_start.elapsed().as_micros() as u32;
        let others_start = std::time::Instant::now();

        // move cam
        self.inputs.move_transform(
            self.transforms
                .get_transform_mut(&self.camera.transform)
                .unwrap(),
            seconds_passed,
        );

        // update rotate
        let mut query = <(&TransformID, &Rotate)>::query();
        for (transform_id, rotate) in query.iter(&self.world) {
            let transform = self.transforms.get_transform_mut(transform_id).unwrap();
            transform.set_rotation(
                Quaternion::from_axis_angle(rotate.0, rotate.1 * seconds_passed)
                    * transform.get_local_transform().rotation,
            );
        }
        
        // Translate in straight line
        // Translate (speed, startposition, endposition, direction)
        // true is start to end, false is opposite
        let mut query = <(&TransformID, &mut Translate)>::query();
        for (transform_id, translate) in query.iter_mut(&mut self.world){
            let transform  = self.transforms.get_transform_mut(transform_id).unwrap();
            
            if translate.direction && (transform.get_local_transform().translation - translate.end).dot(translate.end - translate.start) > 0. {
                translate.direction = false;
            }

            if !translate.direction && (transform.get_local_transform().translation - translate.start).dot(translate.start - translate.end) > 0. {
                translate.direction = true;
            }

            let movement =  if translate.direction { transform.get_local_transform().translation + (translate.end - translate.start).normalize() * translate.speed * self.last_delta_time}
            else {transform.get_local_transform().translation - (translate.end - translate.start).normalize() * translate.speed * self.last_delta_time};
            
            transform.set_translation(movement);
        }


        let mut query = <(&TransformID, &TransformTracker)>::query();
        for (transform_id, TransformTracker(tag)) in query.iter(&self.world) {
            let model = self.transforms.get_global_model(transform_id).unwrap();
            println!("[Transform] {}: {:?}", tag, model);
        }

        unsafe {
            let mut profiler = LOGIC_PROFILER.lock().unwrap();
            profiler.add_sample(phys_time, 1);
            profiler.add_sample(coll_time, 2);
            profiler.add_sample(lerp_time, 3);
            profiler.add_sample(others_start.elapsed().as_micros() as u32, 4);
        }
    }

    /// clear the world and transforms and reset the camera
    pub fn clear(&mut self) {
        *self = Self::new();
        // self.world.clear();
        // self.transforms = TransformSystem::new();
        // self.camera = Camera::from_transform(self.transforms.next().unwrap());
        // self.world.push((self.camera.transform,));
    }
}
impl Default for GameWorld {
    fn default() -> Self {
        Self::new()
    }
}
