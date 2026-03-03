use std::{
    f32::consts::PI,
    iter::zip,
    sync::{Arc, RwLock},
};

use cgmath::{InnerSpace, Quaternion, Rad, Rotation3, Vector3};

use crate::{
    game_objects::{
        light::PointLightComponent, transform::TransformCreateInfo, MaterialSwapper, Rotate, Translate, 
        TransformTracker, WorldLoader,
    },
    load_object_with_transform,
    physics::{CuboidCollider, RigidBody},
    render::resource_manager::{MaterialID::*, MeshID::*, TextureID},
};

// type Mesh = std::sync::Arc<crate::vulkano_objects::buffers::MeshBuffers<VertexFull>>;

pub fn init_world(mut loader: WorldLoader) {
    // meshes
    let ina_meshes = [InaBody, InaCloth, InaHair, InaHead];
    let le_meshes = (0..45u8).map(LostEmpire);

    // materials
    let ina_mats = [
        TextureID::InaBody,
        TextureID::InaCloth,
        TextureID::InaHair,
        TextureID::InaHead,
    ]
    .map(Texture);
    // colored materials
    let red_mat = loader
        .resources
        .load_solid_material([1., 0., 0., 1.], false)
        .0;
    let blue_mat = loader
        .resources
        .load_solid_material([0., 0., 1., 1.], false)
        .0;
    let green_mat = loader
        .resources
        .load_solid_material([0., 1., 0., 1.], true)
        .0;

    // objects
    //      Suzanne
    let suzanne_obj = loader.resources.load_ro(Suzanne, UV, true);
    let rotate = Rotate(Vector3::new(1.0, 1.0, 0.0).normalize(), Rad(5.0));
    loader.add_2_comp([0., 0., 0.], suzanne_obj, rotate);

    //      Spam Suzanne
    for x in 0..20 {
        for z in 0..20 {
            let mat = [ina_mats[1], green_mat][(x + z) % 2];
            loader.quick_ro([(x * 3) as f32, 21.0, (z * 3) as f32], Suzanne, mat, true);
        }
    }

    //      Squares
    let obj = loader.resources.load_ro(Square, Gradient, false);
    for (x, y, z) in [(1., 0., 0.), (0., 1., 0.), (0., 0., 1.)] {
        loader.add_1_comp([x, y, z], obj.clone());
    }

    //      Ina
    let rotate = Rotate([0., 1., 0.].into(), Rad(0.5));
    let (ina_transform, _) = loader.add_1_comp([0.0, 15.0, -3.0], rotate);
    for (mesh, mat) in zip(ina_meshes, ina_mats) {
        loader.quick_ro(
            TransformCreateInfo::from_parent(ina_transform),
            mesh,
            mat,
            true,
        );
    }

    //      lost empires
    let le_transform = loader
        .world
        .transforms
        .add_transform(TransformCreateInfo::default());
    for mesh in le_meshes {
        let le_obj = loader
            .resources
            .load_ro(mesh, Texture(TextureID::LostEmpire), true);
        let mat_swapper = MaterialSwapper::new(
            [
                (Texture(TextureID::LostEmpire), true),
                (Texture(TextureID::LostEmpire), false),
                (UV, false),
                (ina_mats[1], true),
            ]
            .map(|(id, lit)| loader.resources.get_material(id, lit)),
        );

        loader.add_2_comp(
            TransformCreateInfo::from_parent(le_transform),
            le_obj,
            mat_swapper,
        );
    }

    // lights
    let ro = loader.resources.load_ro(Cube, red_mat, false);
    loader.add_2_comp(
        TransformCreateInfo::from([0., 15., -3.]).set_scale([0.1, 0.1, 0.1]),
        PointLightComponent::new([1., 0., 0., 3.], 9.),
        ro,
    );
    let ro = loader.resources.load_ro(Cube, blue_mat, false);
    loader.add_2_comp(
        TransformCreateInfo::from([0.0, 18.0, -1.5]).set_scale([0.1, 0.1, 0.1]),
        PointLightComponent::new([0., 0., 1., 2.], 9.),
        ro,
    );

    // spam lights
    let ro = loader.resources.load_ro(Cube, red_mat, false);
    for x in 0..20 {
        for z in -10..10 {
            loader.add_2_comp(
                TransformCreateInfo::from([(x * 3) as f32, 18.3, (z * 3) as f32])
                    .set_scale([0.1, 0.1, 0.1]),
                PointLightComponent::new([1., 0., 0., 1.], 3.),
                ro.clone(),
            );
        }
    }
}

/// Empty scene with just the lost empire map
pub fn init_ui_test(mut loader: WorldLoader) {
    for i in 0..45u8 {
        loader.quick_ro(
            [0., 0., 0.],
            LostEmpire(i),
            Texture(TextureID::LostEmpire),
            true,
        );
    }
}

/// Large plane + cube
pub fn init_phys_test(mut loader: WorldLoader) {
    // let plane_mesh = resources.get_mesh(Square);
    // let cube_mesh = resources.get_mesh(Cube);

    // plane
    let yellow_mat = loader
        .resources
        .load_solid_material([1., 1., 0., 1.], true)
        .0;
    let green_mat = loader
        .resources
        .load_solid_material([0., 1., 0., 1.], true)
        .0;
    let red_mat = loader
        .resources
        .load_solid_material([1., 0., 0., 1.], true)
        .0;
    let blue_mat = loader
        .resources
        .load_solid_material([0., 0., 1., 1.], true)
        .0;

    // plane collider
    let plane_trans = TransformCreateInfo {
        rotation: Quaternion::from_axis_angle([1., 0., 0.].into(), Rad(-PI / 2.)),
        scale: [10., 10., 1.].into(),
        ..Default::default()
    };
    loader.quick_ro(plane_trans, Square, yellow_mat, true);

    let transform_info = TransformCreateInfo::default()
        .set_translation([0., -0.5, 0.])
        .set_scale([5., 0.5, 5.]);
    let transform = loader.world.transforms.add_transform(transform_info);
    let collider = loader.world.colliders.add(
        CuboidCollider::new(transform, None),
        &mut loader.world.transforms,
    );
    load_object_with_transform!(loader.world.world, transform, collider);

    // slope collider (0.1 rad)
    let transform_info = TransformCreateInfo::default()
        .set_translation([10., -0.5, 0.])
        .set_scale([5., 0.5, 5.])
        .set_rotation(Quaternion::from_axis_angle((1., 0., 0.).into(), Rad(0.1)));
    let transform = loader.world.transforms.add_transform(transform_info);
    let collider = loader.world.colliders.add(
        CuboidCollider::new(transform, None),
        &mut loader.world.transforms,
    );
    let ro = loader.resources.load_ro(Cube, yellow_mat, true);
    load_object_with_transform!(loader.world.world, transform, collider, ro);

    // slope collider (0.2 rad)
    let transform_info = TransformCreateInfo::default()
        .set_translation([20., -0.5, 0.])
        .set_scale([5., 0.5, 5.])
        .set_rotation(Quaternion::from_axis_angle((1., 0., 0.).into(), Rad(0.2)));
    let transform = loader.world.transforms.add_transform(transform_info);
    let collider = loader.world.colliders.add(
        CuboidCollider::new(transform, None),
        &mut loader.world.transforms,
    );
    let ro = loader.resources.load_ro(Cube, yellow_mat, true);
    load_object_with_transform!(loader.world.world, transform, collider, ro);

    // axis
    loader.quick_ro(
        TransformCreateInfo::from((1., -10., 0.)).set_scale((0.1, 0.1, 0.1)),
        Cube,
        red_mat,
        true,
    );
    loader.quick_ro(
        TransformCreateInfo::from((0., -9., 0.)).set_scale((0.1, 0.1, 0.1)),
        Cube,
        green_mat,
        true,
    );
    loader.quick_ro(
        TransformCreateInfo::from((0., -10., 1.)).set_scale((0.1, 0.1, 0.1)),
        Cube,
        blue_mat,
        true,
    );

    // rigidbody test
    let t = loader.world.transforms.add_transform([0., 1., 0.]);
    let ro = loader.resources.load_ro(Cube, green_mat, true);
    let mut rb = RigidBody::new(t);
    rb.velocity = (1.0, 10.0, 0.0).into();
    rb.bivelocity = (0.0, 0.0, -5.0).into();
    rb.inv_mass = 0.5;
    rb.set_moi_as_cuboid((1., 1., 1.).into());
    let rb = Arc::new(RwLock::new(rb));
    let collider = loader.world.colliders.add(
        CuboidCollider::new(t, Some(rb.clone())),
        &mut loader.world.transforms,
    );
    // println!("[DEBUG] rb id: {:?}", t);
    load_object_with_transform!(loader.world.world, t, ro, rb, collider);

    // moving collider
    let (pivot, _) = loader.add_1_comp([0., 0., 0.], Rotate([0., 1., 0.].into(), Rad(0.5)));

    let mover = loader
        .world
        .transforms
        .add_transform(TransformCreateInfo::from([9., 0., 0.]).set_parent(Some(pivot)));
    let collider = loader.world.colliders.add(
        CuboidCollider::new(mover, None),
        &mut loader.world.transforms,
    );
    let ro = loader.resources.load_ro(Cube, green_mat, true);
    load_object_with_transform!(loader.world.world, mover, collider, ro);

    // collider test
    let transform = loader.world.transforms.add_transform([0., 5., 0.]);
    let mut rigidbody = RigidBody::new(transform);
    rigidbody.gravity_multiplier = 0.0;
    rigidbody.set_moi_as_cuboid((1., 1., 1.).into());
    let rigidbody = Arc::new(RwLock::new(rigidbody));
    let collider = loader.world.colliders.add(
        CuboidCollider::new(transform, Some(rigidbody.clone())),
        &mut loader.world.transforms,
    );
    let ro = loader.resources.load_ro(Cube, red_mat, true);
    load_object_with_transform!(
        loader.world.world,
        transform,
        collider,
        ro,
        rigidbody // , TransformTracker("prefab")
    );

    let transform = loader.world.transforms.add_transform(
        TransformCreateInfo::from([1.9, 5.4, 1.9]).set_rotation(Quaternion::from_axis_angle(
            [(0.5f32).sqrt(), 0., (0.5f32).sqrt()].into(),
            Rad(PI / 3.),
        )),
    );
    let mut rigidbody = RigidBody::new(transform);
    rigidbody.gravity_multiplier = 0.0;
    rigidbody.set_moi_as_cuboid((1., 1., 1.).into());
    let rigidbody = Arc::new(RwLock::new(rigidbody));
    let collider = loader.world.colliders.add(
        CuboidCollider::new(transform, Some(rigidbody.clone())),
        &mut loader.world.transforms,
    );
    let ro = loader.resources.load_ro(Cube, red_mat, true);
    load_object_with_transform!(loader.world.world, transform, collider, ro, rigidbody);
}


pub fn init_brainrot_test(mut loader: WorldLoader) {
    loader.quick_ro(TransformCreateInfo::default().set_translation([-2.5, -2.5, -55.]).set_rotation(Quaternion::from_angle_y(Rad(PI))), Sixsevenkid, Texture(TextureID::Kid67), true);

    let grey_mat = loader
        .resources
        .load_solid_material([0.5, 0.5, 0.5, 1.], true)
        .0;
    
    

    let ro = loader.resources.load_ro( RightHand, grey_mat, true);
    let translate = Translate::new(3., [0., 0., 0.], [5., 5., 5.]);
    loader.add_2_comp([0., 0., 0.], ro, translate);
}   