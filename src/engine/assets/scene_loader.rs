use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;

use crate::assets::{HMaterial, HShader, HTexture, Material, Mesh, H};
use crate::core::{Bones, GameObjectId, Vertex3D};
use crate::drawables::MeshRenderer;
use crate::utils::iter::{extend_data, interpolate_zeros};
use crate::utils::ExtraMatrixMath;
use crate::World;
use itertools::{izip, Itertools};
use log::warn;
use nalgebra::{Matrix4, Vector2, Vector3};
use russimp_ng::material::{DataContent, MaterialProperty, PropertyTypeInfo, TextureType};
use russimp_ng::node::Node;
use russimp_ng::scene::{PostProcess, Scene};
use russimp_ng::Vector3D;

const POST_STEPS: &[PostProcess] = &[
    PostProcess::CalculateTangentSpace,
    PostProcess::Triangulate,
    PostProcess::SortByPrimitiveType,
    PostProcess::JoinIdenticalVertices,
    PostProcess::GenerateUVCoords,
    PostProcess::GenerateNormals,
    PostProcess::ForceGenerateNormals,
    PostProcess::EmbedTextures,
    PostProcess::LimitBoneWeights,
];

#[allow(dead_code)]
pub struct SceneLoader;

#[allow(dead_code)]
impl SceneLoader {
    pub fn load(world: &mut World, path: &str) -> Result<GameObjectId, Box<dyn Error>> {
        let mut scene = Self::load_scene(path)?;

        let root = match &scene.root {
            Some(node) => node.clone(),
            None => return Ok(world.new_object("Empty Scene")),
        };

        let materials = load_materials(&scene, world);
        update_material_indices(&mut scene, materials);

        let root_object = Self::spawn_object(world, &scene, &root);
        Ok(root_object)
    }

    pub fn load_scene(path: &str) -> Result<Scene, Box<dyn Error>> {
        let scene = Scene::from_file(path, POST_STEPS.to_vec())?;

        Ok(scene)
    }

    pub fn load_scene_from_buffer(model: &[u8], hint: &str) -> Result<Scene, Box<dyn Error>> {
        let scene = Scene::from_buffer(model, POST_STEPS.to_vec(), hint)?;

        Ok(scene)
    }

    pub fn spawn_object(world: &mut World, scene: &Scene, node: &Node) -> GameObjectId {
        let mut node_obj = build_object(world, scene, node);

        for child in node.children.borrow().iter() {
            let child_obj = Self::spawn_object(world, scene, child);
            node_obj.add_child(child_obj);
        }

        node_obj
    }

    pub fn load_first_mesh_from_buffer(
        model: &[u8],
        hint: &str,
    ) -> Result<Option<Mesh>, Box<dyn Error>> {
        let scene = Self::load_scene_from_buffer(model, hint)?;
        Ok(Self::load_first_from_scene(&scene))
    }

    pub fn load_first_mesh(path: &str) -> Result<Option<Mesh>, Box<dyn Error>> {
        let scene = Self::load_scene(path)?;
        Ok(Self::load_first_from_scene(&scene))
    }

    pub fn load_first_from_scene(scene: &Scene) -> Option<Mesh> {
        let mesh = load_first_from_node(&scene, scene.root.as_ref()?, 0);
        mesh
    }

    pub fn load_mesh(scene: &Scene, node: &Node) -> Option<Mesh> {
        if node.meshes.is_empty() {
            return None;
        }

        let mut bones = Bones::default();

        let mut positions: Vec<Vector3<f32>> = Vec::new();
        let mut tex_coords: Vec<Vector2<f32>> = Vec::new();
        let mut normals: Vec<Vector3<f32>> = Vec::new();
        let mut tangents: Vec<Vector3<f32>> = Vec::new();
        let mut bitangents: Vec<Vector3<f32>> = Vec::new();
        let mut bone_idxs: Vec<Vec<u32>> = Vec::new();
        let mut bone_weights: Vec<Vec<f32>> = Vec::new();

        let mut material_ranges = Vec::new();
        let mut mesh_vertex_count_start: usize = 0;

        for mesh_id in node.meshes.iter() {
            let mesh = scene.meshes.get(*mesh_id as usize)?;
            let mut mesh_vertex_count = mesh_vertex_count_start;
            // filter faces with not 3 vertices.
            // 1 and 2 are point and line faces which I can't render yet
            // 3+ shouldn't happen because of Triangulate PostProcess Assimp feature, thanksyu
            let filtered_faces = mesh.faces.iter().filter(|face| face.0.len() == 3);

            for face in filtered_faces.clone() {
                let face_indices = &face.0;

                extend_data(
                    &mut positions,
                    face_indices,
                    &mesh.vertices,
                    vec3_from_vec3d,
                );

                if let Some(Some(dif_tex_coords)) = mesh.texture_coords.first() {
                    extend_data(
                        &mut tex_coords,
                        face_indices,
                        dif_tex_coords,
                        vec2_from_vec3d,
                    );
                } else {
                    warn!(
                        "Face in Mesh {} didn't have any texture coordinates set",
                        mesh_id
                    );
                }

                extend_data(&mut normals, face_indices, &mesh.normals, vec3_from_vec3d);
                extend_data(&mut tangents, face_indices, &mesh.tangents, vec3_from_vec3d);
                extend_data(
                    &mut bitangents,
                    face_indices,
                    &mesh.bitangents,
                    vec3_from_vec3d,
                );

                mesh_vertex_count += 3;
            }

            bone_idxs.resize_with(mesh_vertex_count, Vec::new);
            bone_weights.resize_with(mesh_vertex_count, Vec::new);

            map_bones(
                bone_idxs[mesh_vertex_count_start..mesh_vertex_count].as_mut(),
                bone_weights[mesh_vertex_count_start..mesh_vertex_count].as_mut(),
                &mut bones,
                mesh,
                filtered_faces,
            );

            // TODO: Change material ranges to usize? (This is a question)
            //   Rev1: Not sure, i don't think that's needed
            material_ranges.push((
                H::new(mesh.material_index),
                mesh_vertex_count_start as u32..mesh_vertex_count as u32,
            ));

            mesh_vertex_count_start = mesh_vertex_count;
        }

        if positions.is_empty() {
            return None;
        }

        // it do work tho
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
            positions,
            tex_coords,
            normals,
            tangents,
            bitangents,
            &bone_idxs,
            &bone_weights
        )
        .map(Vertex3D::from)
        .collect();

        let mesh = Mesh::builder(vertices)
            .with_many_textures(material_ranges)
            .with_bones(bones)
            .build();

        Some(mesh)
    }
}

fn load_first_from_node(scene: &Scene, node: &Node, iter: u32) -> Option<Mesh> {
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

fn map_bones<'a>(
    indices: &mut [Vec<u32>], // start from 0 to how many were defined in the raw mesh, not including point or line "faces"
    weights: &mut [Vec<f32>], // "
    mapped: &mut Bones, // are the total bones that this merged mesh will have, mapped to the merged vertices
    raw: &'a russimp_ng::mesh::Mesh, // the raw bones for this specific mesh part
    faces: impl Iterator<Item = &'a russimp_ng::face::Face>, // the faces that need to be mapped
) {
    // grab bones length before so we know where we base our ids off of. the new indices should
    // now be bones_base + raw id
    let original_bone_count = mapped.raw.len();

    // map bones from raw into mapped bone list
    for bone in &raw.bones {
        mapped.names.push(bone.name.clone());
        mapped.raw.push(bone.into())
    }

    // transpose the mapping from `Vec<weights.vertex ids>` to `vertex ids -> Vec<weights>`
    let mapped_indices: HashMap<u32, Vec<(usize, f32)>> = raw
        .bones
        .iter()
        .enumerate()
        .flat_map(|(i, b)| {
            b.weights
                .iter()
                .map(move |w| (w.vertex_id, (i + original_bone_count, w.weight)))
        })
        .into_group_map();

    // map the bone weights to these face vertex ids.
    for face in faces {
        for v_id in &face.0 {
            let Some(idxs) = indices.get_mut(*v_id as usize) else {
                unreachable!("The indices should've been prepared.");
            };
            let Some(weights) = weights.get_mut(*v_id as usize) else {
                unreachable!("The weights should've been prepared.");
            };

            let Some(bones_by_index) = mapped_indices.get(v_id) else {
                // no bones found for this index
                break;
            };

            idxs.extend(bones_by_index.iter().map(|b| b.0 as u32));
            weights.extend(bones_by_index.iter().map(|b| b.1));
        }
    }
}

fn load_texture(
    world: &mut World,
    texture: Rc<RefCell<russimp_ng::material::Texture>>,
) -> HTexture {
    // TODO: Don't load textures that were loaded before and are just shared between two materials
    let texture = texture.borrow();
    match &texture.data {
        DataContent::Texel(_) => panic!("I CAN'T ADD TEXLESLSSE YET PLS HELP"),
        DataContent::Bytes(data) => world
            .assets
            .textures
            .load_image_from_memory(data)
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
                if arr.len() == 3 {
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

    let diffuse = extract_vec3_property(&material.properties, "diffuse", || {
        Vector3::new(0.788, 0.788, 0.788)
    });
    let diffuse_tex = material.textures.get(&TextureType::Diffuse);
    let diffuse_tex_id = diffuse_tex.map(|tex| load_texture(world, tex.clone()));

    let normal_tex = material.textures.get(&TextureType::Normals);
    let normal_tex_id = normal_tex.map(|tex| load_texture(world, tex.clone()));

    let shininess = get_float_property_or(&material.properties, "shininess", 0.0);
    let new_material = Material {
        name,
        color: diffuse,
        shininess,
        diffuse_texture: diffuse_tex_id,
        normal_texture: normal_tex_id,
        shininess_texture: None,
        opacity: 1.0,
        shader: HShader::DIM3,
    };
    world.assets.materials.add(new_material)
}

pub fn build_object(world: &mut World, scene: &Scene, node: &Node) -> GameObjectId {
    let mut node_obj = world.new_object(&node.name);

    if let Some(mesh) = SceneLoader::load_mesh(scene, node) {
        let handle = world.assets.meshes.add(mesh);

        node_obj.drawable = Some(MeshRenderer::new(handle));
    }

    let t = node.transformation;
    let (position, rotation, scale) = Matrix4::from([
        [t.a1, t.b1, t.c1, t.d1],
        [t.a2, t.b2, t.c2, t.d2],
        [t.a3, t.b3, t.c3, t.d3],
        [t.a4, t.b4, t.c4, t.d4],
    ])
    .decompose(); // convert row to column major (assimp to cgmath)

    node_obj.transform.set_local_position_vec(position);
    node_obj.transform.set_local_rotation(rotation);
    node_obj.transform.set_nonuniform_local_scale(scale);

    node_obj
}

fn load_materials(scene: &Scene, world: &mut World) -> HashMap<u32, HMaterial> {
    let mut mapping = HashMap::new();
    for (i, material) in scene.materials.iter().enumerate() {
        let mat_id = load_material(world, material);
        mapping.insert(i as u32, mat_id);
    }
    mapping
}

fn update_material_indices(scene: &mut Scene, mat_map: HashMap<u32, HMaterial>) {
    for mesh in &mut scene.meshes {
        let mapped_mat = mat_map
            .get(&mesh.material_index)
            .cloned()
            .unwrap_or(HMaterial::FALLBACK);
        mesh.material_index = mapped_mat.id();
    }
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
            PropertyTypeInfo::FloatArray(f) => f.first().cloned().unwrap_or(default),
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
