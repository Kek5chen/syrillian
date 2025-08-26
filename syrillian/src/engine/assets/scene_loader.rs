use log::trace;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;

use crate::assets::{HMaterial, HShader, HTexture, Material, Mesh, StoreType, Texture};
use crate::components::{
    AnimationComponent, MeshRenderer, PointLightComponent, SkeletalComponent, SpotLightComponent,
};
use crate::core::{Bones, GameObjectId, Vertex3D};
use crate::rendering::lights::Light;
use crate::utils::animation::{AnimationClip, Channel, TransformKeys};
use crate::utils::iter::interpolate_zeros;
use crate::utils::{light_range, ExtraMatrixMath, VecCompat};
use crate::World;
use itertools::izip;
use log::warn;
use nalgebra::{Matrix4, Quaternion, UnitQuaternion, Vector2, Vector3};
use russimp_ng::light::LightSourceType;
use russimp_ng::material::{DataContent, MaterialProperty, PropertyTypeInfo, TextureType};
use russimp_ng::node::Node;
use russimp_ng::scene::{PostProcess, Scene};
use russimp_ng::sys::{aiShadingMode_aiShadingMode_PBR_BRDF, aiShadingMode_aiShadingMode_Unlit};
use russimp_ng::{Matrix4x4, Vector3D};

const POST_STEPS: &[PostProcess] = &[
    PostProcess::CalculateTangentSpace,
    PostProcess::FindInstances,
    PostProcess::Triangulate,
    PostProcess::SortByPrimitiveType,
    PostProcess::GenerateNormals,
    PostProcess::GenerateUVCoords,
    PostProcess::EmbedTextures,
    PostProcess::LimitBoneWeights,
];

pub struct SceneLoader;

#[rustfmt::skip]
fn mat4_from_assimp(m: &Matrix4x4) -> Matrix4<f32> {
    Matrix4::new(
        m.a1, m.a2, m.a3, m.a4,
        m.b1, m.b2, m.b3, m.b4,
        m.c1, m.c2, m.c3, m.c4,
        m.d1, m.d2, m.d3, m.d4,
    )
}

// Build a name->(parent_name, local_matrix) map for the scene graph.
fn build_node_map<'a>(
    node: &'a Node,
    parent: Option<&'a str>,
    out: &mut HashMap<String, (Option<String>, Matrix4<f32>)>,
) {
    out.insert(
        node.name.clone(),
        (
            parent.map(|s| s.to_string()),
            mat4_from_assimp(&node.transformation),
        ),
    );
    for c in node.children.borrow().iter() {
        build_node_map(c, Some(&node.name), out);
    }
}

/// Dedupe bones by name and remember inverse bind matrices (the Assimp "offset" matrix).
struct BoneTable {
    names: Vec<String>,
    inverse_bind: Vec<Matrix4<f32>>,
    index_of: HashMap<String, usize>,
}

impl BoneTable {
    fn new() -> Self {
        Self {
            names: Vec::new(),
            inverse_bind: Vec::new(),
            index_of: HashMap::new(),
        }
    }

    /// Get global bone index for this name; create if missing.
    fn ensure(&mut self, name: &str, inv_bind: Matrix4<f32>) -> usize {
        if let Some(&i) = self.index_of.get(name) {
            return i;
        }
        let i = self.names.len();
        self.names.push(name.to_string());
        self.inverse_bind.push(inv_bind);
        self.index_of.insert(name.to_string(), i);
        i
    }

    fn into_bones_with_hierarchy(self, scene: &Scene) -> Bones {
        let mut node_map = HashMap::<String, (Option<String>, Matrix4<f32>)>::new();
        if let Some(root) = &scene.root {
            build_node_map(root, None, &mut node_map);
        }

        let mut parents = vec![None; self.names.len()];

        for (i, name) in self.names.iter().enumerate() {
            let mut cur = name.as_str();
            while let Some((parent_name_opt, _)) = node_map.get(cur) {
                if let Some(parent_name) = parent_name_opt {
                    if let Some(&pi) = self.index_of.get(parent_name) {
                        parents[i] = Some(pi);
                        break;
                    }
                    cur = parent_name;
                } else {
                    break;
                }
            }
        }

        Bones {
            names: self.names,
            parents,
            inverse_bind: self.inverse_bind,
            bind_global: Vec::new(), // finalize later
            bind_local: Vec::new(),  // finalize later
            index_of: self.index_of,
        }
    }
}

fn compute_global_transform(
    name: &str,
    node_map: &HashMap<String, (Option<String>, Matrix4<f32>)>,
) -> Matrix4<f32> {
    let mut m = Matrix4::identity();
    let mut cur = Some(name);
    while let Some(cn) = cur {
        if let Some((p, local)) = node_map.get(cn) {
            m = *local * m;
            cur = p.as_deref();
        } else {
            break;
        }
    }
    m
}

fn finalize_bones(scene: &Scene, mesh_node: &Node, bones: &mut Bones) {
    let mut node_map = HashMap::<String, (Option<String>, Matrix4<f32>)>::new();
    if let Some(root) = &scene.root {
        build_node_map(root, None, &mut node_map);
    }

    let mesh_global = compute_global_transform(&mesh_node.name, &node_map);
    let mesh_inv = mesh_global.try_inverse().unwrap_or(Matrix4::identity());

    let n = bones.len();
    bones.bind_global = vec![Matrix4::identity(); n];
    bones.bind_local = vec![Matrix4::identity(); n];

    for (i, name) in bones.names.iter().enumerate() {
        let g_world = compute_global_transform(name, &node_map);
        let g_model = mesh_inv * g_world; // *** convert to mesh MODEL space ***
        bones.bind_global[i] = g_model;
    }
    for i in 0..n {
        bones.bind_local[i] = match bones.parents[i] {
            None => bones.bind_global[i],
            Some(p) => {
                bones.bind_global[p]
                    .try_inverse()
                    .unwrap_or(Matrix4::identity())
                    * bones.bind_global[i]
            }
        };
    }
}

/// Keep top-4 weights, sort descending, renormalize.
fn pack_top4(mut pairs: Vec<(usize, f32)>) -> (Vec<u32>, Vec<f32>) {
    if pairs.is_empty() {
        return (Vec::new(), Vec::new());
    }
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs.truncate(4);
    let sum = pairs.iter().map(|p| p.1).sum::<f32>().max(1e-8);
    let idx: Vec<u32> = pairs.iter().map(|p| p.0 as u32).collect();
    let wts: Vec<f32> = pairs.iter().map(|p| p.1 / sum).collect();
    (idx, wts)
}

fn clip_group_key(anim_name: &str) -> String {
    let mut parts = anim_name.split('|').collect::<Vec<_>>();
    if parts.len() >= 2 {
        let last = parts.pop().unwrap();
        let prev = parts.pop().unwrap();
        format!("{prev}|{last}")
    } else {
        anim_name.to_owned()
    }
}

fn merge_channels(dest: &mut Vec<Channel>, mut src: Vec<Channel>) {
    let mut idx = HashMap::<String, usize>::new();
    for (i, ch) in dest.iter().enumerate() {
        idx.insert(ch.target_name.clone(), i);
    }
    for ch in src.drain(..) {
        if let Some(i) = idx.get(&ch.target_name).copied() {
            dest[i] = ch;
        } else {
            idx.insert(ch.target_name.clone(), dest.len());
            dest.push(ch);
        }
    }
}

fn animations_from_scene(scene: &Scene) -> Vec<AnimationClip> {
    let mut groups: HashMap<String, AnimationClip> = HashMap::new();

    for a in &scene.animations {
        let tps = if a.ticks_per_second > 0.0 {
            a.ticks_per_second as f32
        } else {
            25.0
        };
        let dur = (a.duration as f32) / tps;
        let key = clip_group_key(&a.name);

        let mut channels = Vec::<Channel>::new();
        for ch in &a.channels {
            let mut keys = TransformKeys::default();

            for k in &ch.position_keys {
                keys.t_times.push((k.time as f32) / tps);
                keys.t_values
                    .push(Vector3::new(k.value.x, k.value.y, k.value.z));
            }
            for k in &ch.rotation_keys {
                keys.r_times.push((k.time as f32) / tps);
                keys.r_values
                    .push(UnitQuaternion::from_quaternion(Quaternion::new(
                        k.value.w, k.value.x, k.value.y, k.value.z,
                    )));
            }
            for k in &ch.scaling_keys {
                keys.s_times.push((k.time as f32) / tps);
                keys.s_values
                    .push(Vector3::new(k.value.x, k.value.y, k.value.z));
            }

            channels.push(Channel {
                target_name: ch.name.clone(),
                keys,
            });
        }

        groups
            .entry(key.clone())
            .and_modify(|clip| {
                clip.duration = clip.duration.max(dur);
                merge_channels(&mut clip.channels, channels.clone());
            })
            .or_insert(AnimationClip {
                name: key,
                duration: dur.max(0.0),
                channels,
            });
    }

    groups.into_values().collect()
}

impl SceneLoader {
    pub fn load(world: &mut World, path: &str) -> Result<GameObjectId, Box<dyn Error>> {
        let scene = Self::load_scene(path)?;

        let root = match &scene.root {
            Some(node) => node.clone(),
            None => return Ok(world.new_object("Empty Scene")),
        };

        let materials = load_materials(&scene, world);
        trace!("Loaded materials for {path:?}");

        let root_object = Self::spawn_deep_object(world, &scene, &root, Some(&materials));
        SceneLoader::load_animations(&scene, root_object);
        Ok(root_object)
    }

    fn load_animations(scene: &Scene, mut object: GameObjectId) {
        let clips = animations_from_scene(&scene);
        if !clips.is_empty() {
            let mut anim = object.add_component::<AnimationComponent>();
            anim.set_clips(clips);
            anim.play_index(0, true, 1.0, 1.0);
        }
    }

    pub fn load_scene(path: &str) -> Result<Scene, Box<dyn Error>> {
        trace!("Started loading {path:?}");
        let scene = Scene::from_file(path, POST_STEPS.to_vec())?;
        trace!("Finished parsing {path:?}");

        Ok(scene)
    }

    pub fn load_scene_from_buffer(model: &[u8], hint: &str) -> Result<Scene, Box<dyn Error>> {
        trace!("Start loading scene from memory");
        let scene = Scene::from_buffer(model, POST_STEPS.to_vec(), hint)?;

        Ok(scene)
    }

    pub fn spawn_deep_object(
        world: &mut World,
        scene: &Scene,
        node: &Node,
        materials: Option<&HashMap<u32, HMaterial>>,
    ) -> GameObjectId {
        struct SpawnData<'a, 'b> {
            world: &'a mut World,
            scene: &'b Scene,
            materials: Option<&'b HashMap<u32, HMaterial>>,
        }

        let mut data = SpawnData {
            world,
            scene,
            materials,
        };

        fn inner(ctx: &mut SpawnData, node: &Node, depth: u32) -> GameObjectId {
            let mut node_obj = SceneLoader::build_object(ctx.world, ctx.scene, node, ctx.materials);

            if depth >= 1000 {
                warn!("Node Object Iteration Depth reached 1000");
                return node_obj;
            }

            for child in node.children.borrow().iter() {
                let child_obj = inner(ctx, child, depth + 1);
                trace!("Loaded new scene object {}", child_obj.name);
                node_obj.add_child(child_obj);
            }

            node_obj
        }

        inner(&mut data, node, 1)
    }

    pub fn load_first_mesh_from_buffer(
        model: &[u8],
        hint: &str,
    ) -> Result<Option<(Mesh, Vec<u32>)>, Box<dyn Error>> {
        let scene = Self::load_scene_from_buffer(model, hint)?;
        Ok(Self::load_first_from_scene(&scene))
    }

    pub fn load_first_mesh(path: &str) -> Result<Option<(Mesh, Vec<u32>)>, Box<dyn Error>> {
        let scene = Self::load_scene(path)?;
        Ok(Self::load_first_from_scene(&scene))
    }

    pub fn load_first_from_scene(scene: &Scene) -> Option<(Mesh, Vec<u32>)> {
        load_first_from_node(&scene, scene.root.as_ref()?, 0)
    }

    pub fn load_mesh(scene: &Scene, node: &Node) -> Option<(Mesh, Vec<u32>)> {
        if node.meshes.is_empty() {
            return None;
        }

        let mut bones_tbl = BoneTable::new();

        let mut positions: Vec<Vector3<f32>> = Vec::new();
        let mut tex_coords: Vec<Vector2<f32>> = Vec::new();
        let mut normals: Vec<Vector3<f32>> = Vec::new();
        let mut tangents: Vec<Vector3<f32>> = Vec::new();
        let mut bitangents: Vec<Vector3<f32>> = Vec::new();
        let mut bone_idxs: Vec<Vec<u32>> = Vec::new();
        let mut bone_wts: Vec<Vec<f32>> = Vec::new();

        let mut material_ranges = Vec::new();
        let mut materials = Vec::new();

        for mesh_id in node.meshes.iter().copied() {
            let amesh = scene.meshes.get(mesh_id as usize)?;
            let start_vertex = positions.len();

            let mut weights_by_vertex: HashMap<u32, Vec<(usize, f32)>> = HashMap::new();
            for b in &amesh.bones {
                let inv_bind = mat4_from_assimp(&b.offset_matrix);
                let gidx = bones_tbl.ensure(&b.name, inv_bind);
                for w in &b.weights {
                    weights_by_vertex
                        .entry(w.vertex_id)
                        .or_default()
                        .push((gidx, w.weight));
                }
            }

            let faces = amesh.faces.iter().filter(|f| f.0.len() == 3);
            let tc0 = amesh.texture_coords.first().and_then(|opt| opt.as_ref());

            for f in faces {
                for &vi in &f.0 {
                    let p = vec3_from_vec3d(&amesh.vertices[vi as usize]);
                    let n = amesh
                        .normals
                        .get(vi as usize)
                        .map(vec3_from_vec3d)
                        .unwrap_or(Vector3::zeros());
                    let t = amesh
                        .tangents
                        .get(vi as usize)
                        .map(vec3_from_vec3d)
                        .unwrap_or(Vector3::zeros());
                    let bt = amesh
                        .bitangents
                        .get(vi as usize)
                        .map(vec3_from_vec3d)
                        .unwrap_or(Vector3::zeros());
                    let uv = tc0
                        .and_then(|tc| tc.get(vi as usize))
                        .map(vec2_from_vec3d)
                        .unwrap_or(Vector2::zeros());

                    positions.push(p);
                    normals.push(n);
                    tangents.push(t);
                    bitangents.push(bt);
                    tex_coords.push(uv);

                    let (idxs, wts) = weights_by_vertex
                        .remove(&vi)
                        .map(pack_top4)
                        .unwrap_or_else(|| (Vec::new(), Vec::new()));

                    bone_idxs.push(idxs);
                    bone_wts.push(wts);
                }
            }

            let end_vertex = positions.len();
            material_ranges.push(start_vertex as u32..end_vertex as u32);
            materials.push(amesh.material_index);
        }

        if positions.is_empty() {
            return None;
        }

        interpolate_zeros(
            positions.len(),
            &mut [
                &mut tex_coords,
                &mut normals,
                &mut tangents,
                &mut bitangents,
            ],
        );

        let vertices: Vec<Vertex3D> = izip!(
            positions, tex_coords, normals, tangents, bitangents, &bone_idxs, &bone_wts
        )
        .map(Vertex3D::from)
        .collect();

        let mut bones: Bones = bones_tbl.into_bones_with_hierarchy(scene);
        finalize_bones(scene, node, &mut bones);

        let mesh = Mesh::builder(vertices)
            .with_many_textures(material_ranges)
            .with_bones(bones)
            .build();

        Some((mesh, materials))
    }

    pub fn build_object(
        world: &mut World,
        scene: &Scene,
        node: &Node,
        scene_materials: Option<&HashMap<u32, HMaterial>>,
    ) -> GameObjectId {
        trace!("Starting to build scene object {:?}", node.name);
        let mut node_obj = world.new_object(&node.name);

        if let Some((mesh, materials)) = SceneLoader::load_mesh(scene, node) {
            Self::load_node_mesh(world, scene_materials, &mut node_obj, mesh, materials);
        }

        let (position, rotation, scale) = mat4_from_assimp(&node.transformation).decompose();

        node_obj.transform.set_local_position_vec(position);
        node_obj.transform.set_local_rotation(rotation);
        node_obj.transform.set_nonuniform_local_scale(scale);

        Self::load_node_light(&scene, node, node_obj);

        node_obj
    }

    fn load_node_mesh(
        world: &mut World,
        scene_materials: Option<&HashMap<u32, HMaterial>>,
        node_obj: &mut GameObjectId,
        mesh: Mesh,
        materials: Vec<u32>,
    ) {
        let has_bones = !mesh.bones.is_empty();
        let handle = world.assets.meshes.add(mesh);
        if let Some(scene_materials) = scene_materials {
            let materials = materials
                .iter()
                .map(|&id| {
                    scene_materials
                        .get(&id)
                        .copied()
                        .unwrap_or(HMaterial::FALLBACK)
                })
                .collect();
            node_obj
                .add_component::<MeshRenderer>()
                .change_mesh(handle, Some(materials));
        } else {
            node_obj
                .add_component::<MeshRenderer>()
                .change_mesh(handle, None);
        }

        if has_bones {
            node_obj.add_component::<SkeletalComponent>();
        }
    }

    fn load_node_light(scene: &Scene, node: &Node, mut obj: GameObjectId) {
        let Some(light) = scene.lights.iter().find(|l| l.name == node.name) else {
            return;
        };

        let color_premultiplied: Vector3<f32> = VecCompat::from(&light.color_diffuse);
        let i_max = color_premultiplied.max().max(f32::EPSILON);
        let color = color_premultiplied / i_max;
        let luminance = 0.2126 * color_premultiplied.x
            + 0.7152 * color_premultiplied.y
            + 0.0722 * color_premultiplied.z;

        let radius_m = {
            let sx = light.size.x.max(0.0);
            let sy = light.size.y.max(0.0);
            let r = sx.max(sy);
            if r > 0.0 { r } else { 0.05 }
        };

        let e_for_range = if luminance > 0.0 { luminance } else { i_max };
        let range = light_range(
            e_for_range,
            light.attenuation_constant,
            light.attenuation_linear,
            light.attenuation_quadratic,
            1.0,
        )
            .unwrap_or_else(|| {
                warn!("Light had 0 or infinite range using 100.0");
                100.0
            });

        if light.light_source_type == LightSourceType::Spot {
            let mut spot = obj.add_component::<SpotLightComponent>();
            let data = spot.data_mut(true);

            data.color = color;
            data.inner_angle = light.angle_inner_cone;
            data.outer_angle = light.angle_outer_cone;
            data.range = range;
            data.radius = radius_m;
            data.intensity = luminance / 100.0;
        } else if light.light_source_type == LightSourceType::Point {
            let mut point = obj.add_component::<PointLightComponent>();
            let data = point.data_mut(true);

            data.color = color;
            data.range = range;
            data.radius = radius_m;
            data.intensity = luminance / 100.0;
        }
    }
}

fn load_first_from_node(scene: &Scene, node: &Node, iter: u32) -> Option<(Mesh, Vec<u32>)> {
    if iter > 1000 {
        return None;
    }

    if let Some(mesh) = SceneLoader::load_mesh(scene, node) {
        return Some(mesh);
    }

    for child in node.children.borrow().iter() {
        if let Some(mesh) = load_first_from_node(scene, child, iter + 1) {
            return Some(mesh);
        }
    }

    None
}

fn load_texture(
    world: &mut World,
    texture: Rc<RefCell<russimp_ng::material::Texture>>,
) -> HTexture {
    // TODO: Don't load textures that were loaded before and are just shared between two materials
    let texture = texture.borrow();
    match &texture.data {
        DataContent::Texel(_) => panic!("I CAN'T ADD TEXLESLSSE YET PLS HELP"),
        DataContent::Bytes(data) => Texture::load_image_from_memory(data)
            .map(|t| t.store(world))
            .unwrap_or_else(|e| {
                warn!("Failed to load texture: {e}. Using fallback texture.");
                HTexture::FALLBACK_DIFFUSE
            }),
    }
}

fn extract_vec3_property<F>(properties: &[MaterialProperty], key: &str, default: F) -> Vector3<f32>
where
    F: Fn() -> Vector3<f32>,
{
    let prop = properties.iter().find(|prop| prop.key.contains(key));
    match prop {
        None => default(),
        Some(prop) => match &prop.data {
            PropertyTypeInfo::FloatArray(arr) => {
                if arr.len() >= 3 {
                    Vector3::new(arr[0], arr[1], arr[2])
                } else {
                    warn!(
                        "Property {} was expected to have 3 values but only had {}",
                        key,
                        arr.len()
                    );
                    default()
                }
            }
            _ => default(),
        },
    }
}

fn load_material(world: &mut World, material: &russimp_ng::material::Material) -> HMaterial {
    let name = get_string_property_or(&material.properties, "name", || "Material".to_string());

    let color = extract_vec3_property(&material.properties, "diffuse", || {
        Vector3::new(0.788, 0.788, 0.788)
    });

    let diffuse_tex = material.textures.get(&TextureType::Diffuse);
    let diffuse_texture = diffuse_tex.map(|tex| load_texture(world, tex.clone()));

    let normal_tex = material.textures.get(&TextureType::Normals);
    let normal_texture = normal_tex.map(|tex| load_texture(world, tex.clone()));

    let roughness_tex = material.textures.get(&TextureType::Roughness);
    let roughness_texture = roughness_tex.map(|tex| load_texture(world, tex.clone()));

    let roughness = get_float_property_or(&material.properties, "roughness", 0.5);
    let metallic = get_float_property_or(&material.properties, "metallic", 0.0);
    let alpha = get_float_property_or(&material.properties, "opacity", 1.0);
    let shading_mode = get_int_property_or(
        &material.properties,
        "shadingm",
        aiShadingMode_aiShadingMode_PBR_BRDF,
    );

    let lit = shading_mode != aiShadingMode_aiShadingMode_Unlit;
    let new_material = Material {
        name,
        color,
        roughness,
        metallic,
        diffuse_texture,
        normal_texture,
        roughness_texture,
        alpha,
        lit,
        cast_shadows: true,
        shader: HShader::DIM3,
    };
    world.assets.materials.add(new_material)
}

fn load_materials(scene: &Scene, world: &mut World) -> HashMap<u32, HMaterial> {
    let mut mapping = HashMap::new();
    for (i, material) in scene.materials.iter().enumerate() {
        let mat_id = load_material(world, material);
        mapping.insert(i as u32, mat_id);
    }
    mapping
}

fn get_string_property_or<F>(properties: &[MaterialProperty], key: &str, default: F) -> String
where
    F: Fn() -> String,
{
    let prop = properties.iter().find(|prop| prop.key.contains(key));
    match prop {
        None => default(),
        Some(prop) => match &prop.data {
            PropertyTypeInfo::String(str) => str.clone(),
            _ => default(),
        },
    }
}

fn get_float_property_or(properties: &[MaterialProperty], key: &str, default: f32) -> f32 {
    let prop = properties.iter().find(|prop| prop.key.contains(key));
    match prop {
        None => default,
        Some(prop) => match &prop.data {
            PropertyTypeInfo::FloatArray(f) => f.first().copied().unwrap_or(default),
            _ => default,
        },
    }
}

fn get_int_property_or(properties: &[MaterialProperty], key: &str, default: u32) -> u32 {
    let prop = properties.iter().find(|prop| prop.key.contains(key));
    match prop {
        None => default,
        Some(prop) => match &prop.data {
            PropertyTypeInfo::Buffer(f) => f.first().map(|b| *b as u32).unwrap_or(default),
            _ => default,
        },
    }
}

fn vec3_from_vec3d(v: &Vector3D) -> Vector3<f32> {
    Vector3::new(v.x, v.y, v.z)
}

fn vec2_from_vec3d(v: &Vector3D) -> Vector2<f32> {
    Vector2::new(v.x, v.y)
}
