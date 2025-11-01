use crate::World;
use crate::assets::{HMaterial, HShader, HTexture, Material, Mesh, StoreType, Texture};
use crate::components::{
    AnimationComponent, MeshRenderer, PointLightComponent, SkeletalComponent, SpotLightComponent,
    SunLightComponent,
};
use crate::core::{Bones, GameObjectId, Vertex3D};
use crate::rendering::lights::Light;
use crate::utils::animation::{AnimationClip, Channel, TransformKeys};
use gltf::animation::util::ReadOutputs;
use gltf::image::Format;
use gltf::json::Value;
use gltf::khr_lights_punctual::Kind;
use gltf::{self, buffer::Data as BufferData, image::Data as ImageData};
use gltf::{Document, Node, mesh};
use itertools::izip;
use log::{trace, warn};
use nalgebra::{Matrix4, Quaternion, UnitQuaternion, Vector2, Vector3};
use std::collections::HashMap;
use std::error::Error;
use syrillian_utils::debug_panic;
use wgpu::TextureFormat;

pub struct GltfScene {
    pub doc: Document,
    pub buffers: Vec<BufferData>,
    pub images: Vec<ImageData>,
}

impl GltfScene {
    pub fn import(path: &str) -> Result<Self, Box<dyn Error>> {
        let (doc, buffers, images) = gltf::import(path)?;
        Ok(Self {
            doc,
            buffers,
            images,
        })
    }
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let (doc, buffers, images) = gltf::import_slice(bytes)?;
        Ok(Self {
            doc,
            buffers,
            images,
        })
    }
}

/// Mesh and its Materials
pub type MeshData = Option<(Mesh, Vec<u32>)>;

pub struct SceneLoader;

impl SceneLoader {
    pub fn load(world: &mut World, path: &str) -> Result<GameObjectId, Box<dyn Error>> {
        let scene = GltfScene::import(path)?;
        Self::load_into_world(world, &scene)
    }

    pub fn load_buffer(world: &mut World, model: &[u8]) -> Result<GameObjectId, Box<dyn Error>> {
        let scene = Self::load_scene_from_buffer(model)?;
        Self::load_into_world(world, &scene)
    }

    pub fn load_scene_from_buffer(model: &[u8]) -> Result<GltfScene, Box<dyn Error>> {
        GltfScene::from_slice(model)
    }

    pub fn load_first_mesh(path: &str) -> Result<MeshData, Box<dyn Error>> {
        let scene = GltfScene::import(path)?;
        Ok(Self::load_first_from_scene(&scene))
    }

    pub fn load_first_mesh_from_buffer(model: &[u8]) -> Result<MeshData, Box<dyn Error>> {
        let scene = GltfScene::from_slice(model)?;
        Ok(Self::load_first_from_scene(&scene))
    }

    fn load_into_world(
        world: &mut World,
        gltf_scene: &GltfScene,
    ) -> Result<GameObjectId, Box<dyn Error>> {
        let doc = &gltf_scene.doc;

        let root_scene = doc
            .default_scene()
            .or_else(|| doc.scenes().next())
            .ok_or("glTF contains no scenes")?;

        let materials = load_materials(gltf_scene, world);
        trace!("Loaded materials");

        let mut root = world.new_object("glTF Scene");
        for node in root_scene.nodes() {
            let child = Self::spawn_node(world, gltf_scene, node, Some(&materials));
            root.add_child(child);
        }

        Self::load_animations(gltf_scene, root);

        Ok(root)
    }

    fn load_animations(gltf_scene: &GltfScene, mut root: GameObjectId) {
        let clips = animations_from_scene(gltf_scene);
        if !clips.is_empty() {
            let mut anim = root.add_component::<AnimationComponent>();
            anim.set_clips(clips);
            anim.play_index(0, true, 1.0, 1.0);
        }
    }

    pub fn load_first_from_scene(scene: &GltfScene) -> Option<(Mesh, Vec<u32>)> {
        let doc = &scene.doc;
        let scene0 = doc.default_scene().or_else(|| doc.scenes().next())?;
        for node in scene0.nodes() {
            if let Some(m) = load_first_from_node(scene, node) {
                return Some(m);
            }
        }
        None
    }

    fn spawn_node(
        world: &mut World,
        scene: &GltfScene,
        node: Node,
        materials: Option<&HashMap<u32, HMaterial>>,
    ) -> GameObjectId {
        let name = node.name().unwrap_or("Unnamed").to_string();
        trace!("Starting to build scene object {name:?}");
        let mut obj = world.new_object(name);

        if let Some(extras) = node.extras() {
            if let Ok(value) = serde_json::de::from_str::<Value>(extras.get()) {
                if let Value::Object(props) = value {
                    obj.add_properties(props);
                } else {
                    trace!(
                        "Ignored custom property that was not a map when loading node into an object"
                    );
                }
            } else {
                debug_panic!("Custom Property \"{extras}\" couldn't be read");
            }
        }

        if let Some((mesh, mats)) = load_mesh(scene, node.clone()) {
            Self::attach_mesh(world, materials, &mut obj, mesh, mats);
        }

        let (p, r, s) = node.transform().decomposed();
        obj.transform.set_local_position_vec(Vector3::from(p));
        obj.transform
            .set_local_rotation(UnitQuaternion::from_quaternion(Quaternion::from(r)));
        obj.transform.set_nonuniform_local_scale(Vector3::from(s));

        load_node_light(node.clone(), obj);

        for child in node.children() {
            let c = Self::spawn_node(world, scene, child, materials);
            obj.add_child(c);
        }

        obj
    }

    fn attach_mesh(
        world: &mut World,
        scene_materials: Option<&HashMap<u32, HMaterial>>,
        node_obj: &mut GameObjectId,
        mesh: Mesh,
        materials: Vec<u32>,
    ) {
        let has_bones = !mesh.bones.is_empty();
        let handle = world.assets.meshes.add(mesh);

        if let Some(scene_materials) = scene_materials {
            let m = materials
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
                .change_mesh(handle, Some(m));
        } else {
            node_obj
                .add_component::<MeshRenderer>()
                .change_mesh(handle, None);
        }

        if has_bones {
            node_obj.add_component::<SkeletalComponent>();
        }
    }
}

fn load_first_from_node(scene: &GltfScene, node: Node) -> Option<(Mesh, Vec<u32>)> {
    if let Some(m) = load_mesh(scene, node.clone()) {
        return Some(m);
    }
    for c in node.children() {
        if let Some(m) = load_first_from_node(scene, c) {
            return Some(m);
        }
    }
    None
}

fn load_mesh(scene: &GltfScene, node: Node) -> Option<(Mesh, Vec<u32>)> {
    let mesh = node.mesh()?;
    let skin = node.skin();

    let mut positions: Vec<Vector3<f32>> = Vec::new();
    let mut tex_coords: Vec<Vector2<f32>> = Vec::new();
    let mut normals: Vec<Vector3<f32>> = Vec::new();
    let mut tangents: Vec<Vector3<f32>> = Vec::new();
    let mut bitangents: Vec<Vector3<f32>> = Vec::new();
    let mut bone_idxs: Vec<Vec<u32>> = Vec::new();
    let mut bone_wts: Vec<Vec<f32>> = Vec::new();

    let mut ranges = Vec::<std::ops::Range<u32>>::new();
    let mut materials = Vec::<u32>::new();

    let mut bones = Bones::default();
    let mut joint_node_index_of: HashMap<usize, usize> = HashMap::new();
    if let Some(s) = skin {
        build_bones_from_skin(scene, s, node, &mut bones, &mut joint_node_index_of);
    }

    let get_buf = |b: gltf::Buffer| -> Option<&[u8]> { Some(&scene.buffers[b.index()].0) };

    let mut start_vertex = 0u32;
    for prim in mesh.primitives() {
        let reader = prim.reader(get_buf);

        let pos = reader.read_positions()?.collect::<Vec<_>>();
        let nrm = reader.read_normals().map(|it| it.collect::<Vec<_>>());
        let tan = reader.read_tangents().map(|it| it.collect::<Vec<_>>());
        let uv0 = reader.read_tex_coords(0).map(|tc| match tc {
            mesh::util::ReadTexCoords::F32(i) => i.collect::<Vec<_>>(),
            mesh::util::ReadTexCoords::U16(i) => i
                .map(|v| [v[0] as f32 / 65535.0, v[1] as f32 / 65535.0])
                .collect(),
            mesh::util::ReadTexCoords::U8(i) => i
                .map(|v| [v[0] as f32 / 255.0, v[1] as f32 / 255.0])
                .collect(),
        });

        let indices: Vec<u32> = if let Some(ind) = reader.read_indices() {
            ind.into_u32().collect()
        } else {
            (0u32..pos.len() as u32).collect()
        };

        type OptJoints = Option<Vec<[u16; 4]>>;
        type OptWeights = Option<Vec<[f32; 4]>>;

        let (joints, weights): (OptJoints, OptWeights) =
            match (reader.read_joints(0), reader.read_weights(0)) {
                (Some(js), Some(ws)) => {
                    let js = match js {
                        mesh::util::ReadJoints::U8(i) => i
                            .map(|j| [j[0] as u16, j[1] as u16, j[2] as u16, j[3] as u16])
                            .collect(),
                        mesh::util::ReadJoints::U16(i) => i.collect(),
                    };
                    let ws = match ws {
                        mesh::util::ReadWeights::F32(i) => i.collect(),
                        mesh::util::ReadWeights::U16(i) => i
                            .map(|w| {
                                [
                                    w[0] as f32 / 65535.0,
                                    w[1] as f32 / 65535.0,
                                    w[2] as f32 / 65535.0,
                                    w[3] as f32 / 65535.0,
                                ]
                            })
                            .collect(),
                        mesh::util::ReadWeights::U8(i) => i
                            .map(|w| {
                                [
                                    w[0] as f32 / 255.0,
                                    w[1] as f32 / 255.0,
                                    w[2] as f32 / 255.0,
                                    w[3] as f32 / 255.0,
                                ]
                            })
                            .collect(),
                    };
                    (Some(js), Some(ws))
                }
                _ => (None, None),
            };

        if prim.mode() != mesh::Mode::Triangles {
            warn!("Non-triangle primitive encountered; skipping.");
            continue;
        }

        for tri in indices.chunks_exact(3) {
            for &vi in tri {
                let p = pos[vi as usize];
                positions.push(Vector3::new(p[0], p[1], p[2]));

                if let Some(n) = &nrm {
                    let n = n[vi as usize];
                    normals.push(Vector3::new(n[0], n[1], n[2]));
                } else {
                    normals.push(Vector3::zeros());
                }

                if let Some(t) = &tan {
                    let t4 = t[vi as usize];
                    let t3 = Vector3::new(t4[0], t4[1], t4[2]);
                    tangents.push(t3);
                    let n3 = *normals.last().unwrap();
                    let b = n3.cross(&t3).normalize() * t4[3].signum();
                    bitangents.push(b);
                } else {
                    tangents.push(Vector3::zeros());
                    bitangents.push(Vector3::zeros());
                }

                if let Some(uv) = &uv0 {
                    let uv = uv[vi as usize];
                    tex_coords.push(Vector2::new(uv[0], uv[1]));
                } else {
                    tex_coords.push(Vector2::zeros());
                }

                if let (Some(js), Some(ws)) = (&joints, &weights) {
                    let j = js[vi as usize];
                    let w = ws[vi as usize];
                    let idxs = [
                        *joint_node_index_of.get(&(j[0] as usize)).unwrap_or(&0),
                        *joint_node_index_of.get(&(j[1] as usize)).unwrap_or(&0),
                        *joint_node_index_of.get(&(j[2] as usize)).unwrap_or(&0),
                        *joint_node_index_of.get(&(j[3] as usize)).unwrap_or(&0),
                    ];
                    bone_idxs.push(vec![
                        idxs[0] as u32,
                        idxs[1] as u32,
                        idxs[2] as u32,
                        idxs[3] as u32,
                    ]);
                    let s = (w[0] + w[1] + w[2] + w[3]).max(1e-8);
                    bone_wts.push(vec![w[0] / s, w[1] / s, w[2] / s, w[3] / s]);
                } else {
                    bone_idxs.push(Vec::new());
                    bone_wts.push(Vec::new());
                }
            }
        }

        let end = positions.len() as u32;
        ranges.push(start_vertex..end);
        start_vertex = end;

        materials.push(prim.material().index().map(|i| i as u32).unwrap_or(0));
    }

    if positions.is_empty() {
        return None;
    }

    crate::utils::iter::interpolate_zeros(
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

    let mesh = Mesh::builder(vertices)
        .with_many_textures(ranges)
        .with_bones(bones)
        .build();

    Some((mesh, materials))
}

fn build_bones_from_skin(
    scene: &GltfScene,
    skin: gltf::Skin,
    mesh_node: Node,
    out: &mut Bones,
    joint_map: &mut HashMap<usize, usize>,
) {
    let mut names = Vec::<String>::new();
    let mut parents = Vec::<Option<usize>>::new();
    let mut inverse_bind = Vec::<Matrix4<f32>>::new();
    let mut index_of = HashMap::<String, usize>::new();

    let mut node_map = HashMap::<usize, (Option<usize>, Matrix4<f32>)>::new();
    for scene0 in scene.doc.scenes() {
        for n in scene0.nodes() {
            build_node_map_recursive(n, None, &mut node_map);
        }
    }

    let get_buf = |b: gltf::Buffer| -> Option<&[u8]> { Some(&scene.buffers[b.index()].0) };
    let inv_mats: Vec<Matrix4<f32>> = skin
        .reader(get_buf)
        .read_inverse_bind_matrices()
        .map(|iter| iter.map(Matrix4::from).collect())
        .unwrap_or_default();

    for (joint_idx, joint_node) in skin.joints().enumerate() {
        let name = joint_node.name().unwrap_or("<unnamed>").to_string();
        let my_index = names.len();
        names.push(name.clone());
        index_of.insert(name.clone(), my_index);
        joint_map.insert(joint_idx, my_index);

        let parent = node_map
            .get(&joint_node.index())
            .and_then(|(p, _)| *p)
            .and_then(|pi| {
                skin.joints()
                    .position(|jn| jn.index() == pi)
                    .and_then(|local| joint_map.get(&local).copied())
            });
        parents.push(parent);

        let ib = inv_mats
            .get(joint_idx)
            .cloned()
            .unwrap_or_else(Matrix4::identity);
        inverse_bind.push(ib);
    }

    let mesh_global = global_transform_of(mesh_node.index(), &node_map);
    let mesh_global_inv = mesh_global.try_inverse().unwrap_or(Matrix4::identity());

    let mut bind_global = vec![Matrix4::identity(); names.len()];
    for (i, joint_node) in skin.joints().enumerate() {
        let g_world = global_transform_of(joint_node.index(), &node_map);
        bind_global[i] = mesh_global_inv * g_world;
    }

    let mut bind_local = vec![Matrix4::identity(); names.len()];
    for i in 0..names.len() {
        bind_local[i] = match parents[i] {
            None => bind_global[i],
            Some(p) => bind_global[p].try_inverse().unwrap_or(Matrix4::identity()) * bind_global[i],
        };
    }

    let mut children = vec![Vec::new(); names.len()];
    for (i, parent) in parents.iter().enumerate() {
        match *parent {
            None => out.roots.push(i),
            Some(p) => children[p].push(i),
        }
    }

    out.names = names;
    out.parents = parents;
    out.children = children;
    out.inverse_bind = inverse_bind;
    out.bind_global = bind_global;
    out.bind_local = bind_local;
    out.index_of = index_of;
}

fn build_node_map_recursive(
    node: Node,
    parent: Option<usize>,
    out: &mut HashMap<usize, (Option<usize>, Matrix4<f32>)>,
) {
    out.insert(
        node.index(),
        (parent, Matrix4::from(node.transform().matrix())),
    );
    for c in node.children() {
        build_node_map_recursive(c, Some(node.index()), out);
    }
}

fn global_transform_of(
    node_idx: usize,
    node_map: &HashMap<usize, (Option<usize>, Matrix4<f32>)>,
) -> Matrix4<f32> {
    let mut m = Matrix4::identity();
    let mut cur = Some(node_idx);
    while let Some(ci) = cur {
        if let Some((p, local)) = node_map.get(&ci) {
            m = *local * m;
            cur = *p;
        } else {
            break;
        }
    }
    m
}

fn animations_from_scene(scene: &GltfScene) -> Vec<AnimationClip> {
    let mut clips = Vec::<AnimationClip>::new();
    let get_buf = |b: gltf::Buffer| -> Option<&[u8]> { Some(&scene.buffers[b.index()].0) };

    for anim in scene.doc.animations() {
        let name = anim.name().unwrap_or("Animation").to_string();
        let mut channels_out = Vec::<Channel>::new();
        let mut max_time = 0.0f32;

        for ch in anim.channels() {
            let target = ch.target();
            let node = target.node();
            let target_name = node
                .name()
                .unwrap_or(&format!("node{}", node.index()))
                .to_string();

            let reader = ch.reader(get_buf);

            if let Some(times) = reader.read_inputs() {
                let times: Vec<f32> = times.collect();
                max_time = max_time.max(times.last().copied().unwrap_or(0.0));

                let mut keys = TransformKeys::default();
                match reader.read_outputs().expect("outputs") {
                    ReadOutputs::Translations(v) => {
                        let vals: Vec<[f32; 3]> = v.collect();
                        keys.t_times = times.clone();
                        keys.t_values = vals
                            .into_iter()
                            .map(|v| Vector3::new(v[0], v[1], v[2]))
                            .collect();
                    }
                    ReadOutputs::Rotations(v) => {
                        let vals: Vec<_> = v.into_f32().collect();
                        keys.r_times = times.clone();
                        keys.r_values = vals
                            .into_iter()
                            .map(|q| {
                                UnitQuaternion::from_quaternion(Quaternion::new(
                                    q[3], q[0], q[1], q[2],
                                ))
                            })
                            .collect();
                    }
                    ReadOutputs::Scales(v) => {
                        let vals: Vec<[f32; 3]> = v.collect();
                        keys.s_times = times.clone();
                        keys.s_values = vals
                            .into_iter()
                            .map(|v| Vector3::new(v[0], v[1], v[2]))
                            .collect();
                    }
                    _ => {} // TODO: weights
                }

                channels_out.push(Channel { target_name, keys });
            }
        }

        clips.push(AnimationClip {
            name,
            duration: max_time,
            channels: channels_out,
        });
    }

    clips
}

fn load_materials(scene: &GltfScene, world: &mut World) -> HashMap<u32, HMaterial> {
    let mut map = HashMap::new();

    for (i, mat) in scene.doc.materials().enumerate() {
        let name = mat.name().unwrap_or("Material").to_string();
        let pbr = mat.pbr_metallic_roughness();

        let base = pbr.base_color_factor();
        let color = Vector3::new(base[0], base[1], base[2]);
        let alpha = base[3];
        let metallic = pbr.metallic_factor();
        let roughness = pbr.roughness_factor();

        let diffuse_texture = load_texture(scene, world, pbr.base_color_texture());
        let normal_texture = load_texture(scene, world, mat.normal_texture());
        let roughness_texture = load_texture(scene, world, pbr.metallic_roughness_texture());

        let lit = !mat.unlit();

        let new_mat = Material {
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
        map.insert(i as u32, world.assets.materials.add(new_mat));
    }

    map
}

fn load_texture<'a, T>(scene: &'a GltfScene, world: &mut World, info: Option<T>) -> Option<HTexture>
where
    T: AsRef<gltf::texture::Texture<'a>>,
{
    let tex = info.as_ref()?.as_ref();
    let img = tex.source();
    let idx = img.index();

    let pixels = &scene.images[idx].pixels;
    let mut data = Vec::new();
    let (w, h) = (scene.images[idx].width, scene.images[idx].height);

    let original_format = scene.images[idx].format;

    let format = match original_format {
        Format::R8 => TextureFormat::R8Unorm,
        Format::R8G8 => TextureFormat::Rg8Unorm,
        Format::R8G8B8 => TextureFormat::Rgba8UnormSrgb,
        Format::R8G8B8A8 => TextureFormat::Rgba8UnormSrgb,
        Format::R16 => TextureFormat::R16Unorm,
        Format::R16G16 => TextureFormat::Rg16Snorm,
        Format::R16G16B16 => {
            debug_panic!("Cannot use RGB16 (no alpha) Texture");
            return None;
        }
        Format::R16G16B16A16 => TextureFormat::Rgba16Unorm,
        Format::R32G32B32FLOAT => {
            debug_panic!("Cannot use RGB32 (no alpha) Texture");
            return None;
        }
        Format::R32G32B32A32FLOAT => TextureFormat::Rgba32Float,
    };

    if original_format == Format::R8G8B8 {
        for rgb in pixels.chunks(3) {
            data.extend(rgb);
            data.push(255);
        }
    } else {
        data = pixels.clone();
    }

    debug_assert_eq!(
        data.len(),
        w as usize * h as usize * format.block_copy_size(None).unwrap() as usize
    );

    Some(Texture::load_pixels(data, w, h, TextureFormat::Rgba8UnormSrgb).store(world))
}

fn load_node_light(node: Node, mut obj: GameObjectId) {
    if let Some(nl) = node.light() {
        let color = Vector3::new(nl.color()[0], nl.color()[1], nl.color()[2]);
        let intensity = nl.intensity(); // point/spot: candela (lm/sr); directional: lux (lx)
        let range = nl.range().unwrap_or(100.0);
        match nl.kind() {
            Kind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            } => {
                let mut spot = obj.add_component::<SpotLightComponent>();
                let d = spot.data_mut(true);
                d.color = color;
                d.inner_angle = inner_cone_angle;
                d.outer_angle = outer_cone_angle;
                d.range = range;
                d.radius = 0.05;
                d.intensity = intensity;
            }
            Kind::Point => {
                let mut p = obj.add_component::<PointLightComponent>();
                let d = p.data_mut(true);
                d.color = color;
                d.range = range;
                d.radius = 0.05;
                d.intensity = intensity;
            }
            Kind::Directional => {
                let mut p = obj.add_component::<SunLightComponent>();
                let d = p.data_mut(true);
                d.color = color;
                d.range = range;
                d.radius = 0.05;
                d.intensity = intensity;
            }
        }
    }
}
