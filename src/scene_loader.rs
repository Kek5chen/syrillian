use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;

use bytemuck::Contiguous;
use itertools::izip;
use log::warn;
use nalgebra::{Matrix4, Vector2, Vector3};
use num_traits::{ToPrimitive, Zero};
use russimp_ng::material::{DataContent, MaterialProperty, PropertyTypeInfo, TextureType};
use russimp_ng::node::Node;
use russimp_ng::scene::{PostProcess, Scene};
use russimp_ng::Vector3D;
use wgpu::TextureFormat;

use crate::asset_management::materialmanager::{Material, MaterialId};
use crate::asset_management::mesh::{Mesh, Vertex3D};
use crate::asset_management::shadermanager::ShaderId;
use crate::asset_management::texturemanager::{TextureId, FALLBACK_DIFFUSE_TEXTURE};
use crate::asset_management::DIM3_SHADER_ID;
use crate::drawables::mesh_renderer::MeshRenderer;
use crate::object::GameObjectId;
use crate::utils::math::ExtraMatrixMath;
use crate::world::World;

#[allow(dead_code)]
pub struct SceneLoader;

#[allow(dead_code)]
impl SceneLoader {
    pub fn load(world: &mut World, path: &str) -> Result<GameObjectId, Box<dyn Error>> {
        let mut scene = Scene::from_file(
            path,
            vec![
                PostProcess::CalculateTangentSpace,
                PostProcess::Triangulate,
                PostProcess::SortByPrimitiveType,
                PostProcess::JoinIdenticalVertices,
                PostProcess::GenerateUVCoords,
                PostProcess::GenerateNormals,
                PostProcess::ForceGenerateNormals,
                PostProcess::EmbedTextures,
            ],
        )?;

        let root = match &scene.root {
            Some(node) => node.clone(),
            None => return Ok(world.new_object("EmptyLoadedObject")),
        };

        let materials = Self::load_materials(&scene, world);
        Self::update_material_indicies(&mut scene, materials);
        let root_object = world.new_object(&root.name);
        Self::load_rec(world, &scene, &root, root_object);
        Ok(root_object)
    }

    fn load_rec(world: &mut World, scene: &Scene, node: &Rc<Node>, mut node_obj: GameObjectId) {
        Self::load_data(world, scene, node, node_obj);
        for child in node.children.borrow().iter() {
            let obj = world.new_object(&child.name);
            node_obj.add_child(obj);
            Self::load_rec(world, scene, child, obj);
        }
    }

    fn extract_data<T, O, F>(indices: &[u32], source: &[T], converter: F) -> Vec<O>
    where
        F: Fn(&T) -> O,
    {
        indices
            .iter()
            .filter_map(|&idx| source.get(idx as usize).map(&converter))
            .collect()
    }

    fn extend_data<T, O, F>(extendable: &mut Vec<T>, indices: &[u32], source: &[O], converter: F)
    where
        F: Fn(&O) -> T,
    {
        extendable.extend(Self::extract_data(indices, source, converter));
    }

    // hehe, idk either
    fn normalize_data<S, X, Y, Z, A>(
        scalar: &mut [S],
        x: &mut Vec<X>,
        y: &mut Vec<Y>,
        z: &mut Vec<Z>,
        a: &mut Vec<A>,
    ) where
        S: Zero + Clone,
        X: Zero + Clone,
        Y: Zero + Clone,
        Z: Zero + Clone,
        A: Zero + Clone,
    {
        if x.len() != scalar.len() {
            x.resize(scalar.len(), X::zero());
        }

        if y.len() != scalar.len() {
            y.resize(scalar.len(), Y::zero());
        }

        if z.len() != scalar.len() {
            z.resize(scalar.len(), Z::zero());
        }

        if a.len() != scalar.len() {
            a.resize(scalar.len(), A::zero());
        }
    }

    fn load_data(world: &mut World, scene: &Scene, node: &Rc<Node>, node_obj: GameObjectId) {
        if node.meshes.is_empty() {
            return;
        }

        let mut positions: Vec<Vector3<f32>> = Vec::new();
        let mut tex_coords: Vec<Vector2<f32>> = Vec::new();
        let mut normals: Vec<Vector3<f32>> = Vec::new();
        let mut tangents: Vec<Vector3<f32>> = Vec::new();
        let mut bitangents: Vec<Vector3<f32>> = Vec::new();

        const VEC3_FROM_VEC3D: fn(&Vector3D) -> Vector3<f32> =
            |v: &Vector3D| Vector3::new(v.x, v.y, v.z);
        const VEC2_FROM_VEC3D: fn(&Vector3D) -> Vector2<f32> =
            |v: &Vector3D| Vector2::new(v.x, v.y);

        let mut material_ranges = Vec::new();
        let mut mesh_vertex_count_start = 0;
        for (_, mesh) in (0..)
            .zip(scene.meshes.iter())
            .filter(|(i, _)| node.meshes.contains(i))
        {
            let mut mesh_vertex_count = mesh_vertex_count_start;
            for face in &mesh.faces {
                if face.0.len() != 3 {
                    continue; // ignore line and point primitives
                }
                let face_indices = &face.0;
                Self::extend_data(
                    &mut positions,
                    face_indices,
                    &mesh.vertices,
                    VEC3_FROM_VEC3D,
                );
                if let Some(Some(dif_tex_coords)) = mesh.texture_coords.first() {
                    Self::extend_data(
                        &mut tex_coords,
                        face_indices,
                        dif_tex_coords,
                        VEC2_FROM_VEC3D,
                    );
                }
                Self::extend_data(&mut normals, face_indices, &mesh.normals, VEC3_FROM_VEC3D);
                Self::extend_data(&mut tangents, face_indices, &mesh.tangents, VEC3_FROM_VEC3D);
                Self::extend_data(
                    &mut bitangents,
                    face_indices,
                    &mesh.bitangents,
                    VEC3_FROM_VEC3D,
                );
                mesh_vertex_count += 3;
            }
            material_ranges.push((
                mesh.material_index as usize,
                mesh_vertex_count_start..mesh_vertex_count,
            ));
            mesh_vertex_count_start = mesh_vertex_count;
        }

        // it does work tho
        Self::normalize_data(
            &mut positions,
            &mut tex_coords,
            &mut normals,
            &mut tangents,
            &mut bitangents,
        );

        let vertices = izip!(positions, tex_coords, normals, tangents, bitangents)
            .map(
                |(position, tex_coord, normal, tangent, bitangent)| Vertex3D {
                    position,
                    tex_coord,
                    normal,
                    tangent,
                    bitangent,
                },
            )
            .collect();

        let mesh = Mesh::new(vertices, None, Some(material_ranges));
        let id = world.assets.meshes.add_mesh(mesh);

        let mut node_obj = node_obj;
        node_obj.drawable = Some(MeshRenderer::new(id));

        // set transformations
        let t = node.transformation;
        let (position, rotation, scale) = Matrix4::from([
            [t.a1, t.b1, t.c1, t.d1],
            [t.a2, t.b2, t.c2, t.d2],
            [t.a3, t.b3, t.c3, t.d3],
            [t.a4, t.b4, t.c4, t.d4],
        ])
        .decompose(); // convert row to column major (assimp to cgmath)

        node_obj.transform.set_local_position(position);
        node_obj.transform.set_local_rotation(rotation);
        node_obj.transform.set_nonuniform_local_scale(scale);
    }

    fn load_materials(scene: &Scene, world: &mut World) -> HashMap<u32, MaterialId> {
        let mut mapping = HashMap::new();
        for (i, material) in scene.materials.iter().enumerate() {
            let mat_id = Self::load_material(world, material, DIM3_SHADER_ID);
            mapping.insert(i as u32, mat_id);
        }
        mapping
    }
    fn update_material_indicies(scene: &mut Scene, mat_map: HashMap<u32, MaterialId>) {
        for mesh in &mut scene.meshes {
            let new_idx = mat_map.get(&mesh.material_index).cloned();
            let intermediate_idx = new_idx.unwrap_or_default();
            mesh.material_index = intermediate_idx.to_u32().unwrap_or_default();

            if intermediate_idx != mesh.material_index as usize {
                warn!("Scene tried to use more than {} materials. That's crazy and I thought you wanted to know that.", u32::MAX_VALUE);
            }
        }
    }

    fn load_texture(
        world: &mut World,
        texture: Rc<RefCell<russimp_ng::material::Texture>>,
    ) -> TextureId {
        // TODO: Don't load textures that were loaded before and are just shared between two materials
        let texture = texture.borrow();
        match &texture.data {
            DataContent::Texel(_) => panic!("I CAN'T ADD TEXLESLSSE YET PLS HELP"),
            DataContent::Bytes(data) => {
                let decoded = match image::load_from_memory(data) {
                    Ok(decoded) => decoded,
                    Err(e) => {
                        warn!("Failed to load texture: {}. Using fallback texture.", e);
                        return FALLBACK_DIFFUSE_TEXTURE;
                    }
                };

                let rgba = decoded.into_rgba8();
                let mut data = Vec::with_capacity((rgba.width() * rgba.height() * 4) as usize);
                for pixel in rgba.pixels() {
                    data.push(pixel[2]); // B
                    data.push(pixel[1]); // G
                    data.push(pixel[0]); // R
                    data.push(pixel[3]); // A
                }

                world.assets.textures.add_texture(
                    rgba.width(),
                    rgba.height(),
                    TextureFormat::Bgra8UnormSrgb,
                    Some(data),
                )
            }
        }
    }

    fn extract_vec3_property<F>(
        properties: &[MaterialProperty],
        key: &str,
        default: F,
    ) -> Vector3<f32>
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

    fn extract_string_property<F>(properties: &[MaterialProperty], key: &str, default: F) -> String
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

    fn extract_float_property(properties: &[MaterialProperty], key: &str, default: f32) -> f32 {
        let prop = properties.iter().find(|prop| prop.key.contains(key));
        match prop {
            None => default,
            Some(prop) => match &prop.data {
                PropertyTypeInfo::FloatArray(f) => f.first().cloned().unwrap_or(default),
                _ => default,
            },
        }
    }

    fn load_material(
        world: &mut World,
        material: &russimp_ng::material::Material,
        shader: ShaderId,
    ) -> MaterialId {
        let name =
            Self::extract_string_property(&material.properties, "name", || "Material".to_string());

        let diffuse = Self::extract_vec3_property(&material.properties, "diffuse", || {
            Vector3::new(0.788, 0.788, 0.788)
        });
        let diffuse_tex = material.textures.get(&TextureType::Diffuse);
        let diffuse_tex_id = diffuse_tex.map(|tex| Self::load_texture(world, tex.clone()));

        let normal_tex = material.textures.get(&TextureType::Normals);
        let normal_tex_id = normal_tex.map(|tex| Self::load_texture(world, tex.clone()));

        let shininess = Self::extract_float_property(&material.properties, "shininess", 0.0);
        let new_material = Material {
            name,
            diffuse,
            shininess,
            diffuse_texture: diffuse_tex_id,
            normal_texture: normal_tex_id,
            shininess_texture: None,
            opacity: 1.0,
            shader: Some(shader),
        };
        world.assets.materials.add_material(new_material)
    }
}
