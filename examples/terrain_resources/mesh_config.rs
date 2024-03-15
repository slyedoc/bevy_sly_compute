use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
        texture::TextureFormatPixelInfo,
    },
};
use bevy_inspector_egui::{inspector_options::std_options::NumberDisplay, prelude::*};
use bevy_xpbd_3d::{math::Scalar, plugins::collision::Collider};

use crate::TEXTURE_SIZE;

#[derive(Reflect, Resource, InspectorOptions)]
pub struct TerrainMeshConfig {
    /// how big the chunks in world units
    #[inspector(min = 2, max = 500, display = NumberDisplay::Slider)]
    pub world_size: u32,

    #[inspector(min = -1.0, max = 1.0, display = NumberDisplay::Slider)]
    pub height_scale: f32,

    /// number of divisions along one edge of the mesh
    #[inspector(min = 2, max = 100, display = NumberDisplay::Slider)]
    pub mesh_density: u32,

    /// number of divisions along one edge of the height map collider
    #[inspector(min = 2, max = 100, display = NumberDisplay::Slider)]
    pub collider_density: u32,
}

impl Default for TerrainMeshConfig {
    fn default() -> Self {
        Self {
            world_size: 50,
            height_scale: 0.1,
            mesh_density: 100,
            collider_density: 50,
        }
    }
}

impl TerrainMeshConfig {
    pub fn generate_mesh(&self, image: &Image) -> Mesh {
        let pixel_size = image.texture_descriptor.format.pixel_size();

        let chuck_size = self.world_size as f32;
        let half_chuck_size = self.world_size as f32 / 2.0;
        let divisions = (self.mesh_density as usize).max(2);

        let num_vertices = divisions * divisions;
        let num_indices = (divisions - 1) * (divisions - 1) * 6;

        let mut positions: Vec<[f32; 3]> = Vec::with_capacity(num_vertices as usize);
        let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(num_vertices as usize);
        let mut indices: Vec<u32> = Vec::with_capacity(num_indices as usize);

        for y in 0..divisions {
            for x in 0..divisions {
                // Calculate the position in the image
                let img_x = x * (TEXTURE_SIZE - 1) / (divisions - 1);
                let img_y = y * (TEXTURE_SIZE - 1) / (divisions - 1);
                let index = (img_y * TEXTURE_SIZE + img_x) * pixel_size;
                let pixel_data = &image.data[index..(index + pixel_size)];
                let height = pixel_data[0] as f32 / 255.0;
                positions.push([
                    (x as f32 / (divisions - 1) as f32) * chuck_size - half_chuck_size,
                    height * self.world_size as f32 * self.height_scale,
                    (y as f32 / (divisions - 1) as f32) * chuck_size - half_chuck_size,
                ]);

                // Vertex index in the positions array
                uvs.push([
                    x as f32 / (divisions - 1) as f32,
                    y as f32 / (divisions - 1) as f32,
                ]);
                if x < divisions - 1 && y < divisions - 1 {
                    let base = y * divisions + x;
                    indices.extend_from_slice(&[
                        base as u32,
                        (base + divisions) as u32,
                        (base + 1) as u32,
                        (base + 1) as u32,
                        (base + divisions) as u32,
                        (base + divisions + 1) as u32,
                    ]);
                }
            }
        }

        // build our mesh
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_indices(Indices::U32(indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

        // compute flat normals
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.duplicate_vertices();
        mesh.compute_flat_normals();

        mesh
    }

    pub fn generate_collider(&self, image: &Image) -> Collider {
        let pixel_size = image.texture_descriptor.format.pixel_size();
        let divisions = (self.collider_density as usize).max(2);

        let mut heights: Vec<Vec<Scalar>> = Vec::with_capacity(divisions);
        let scale = Vec3::splat(self.world_size as f32);

        for y in 0..divisions {
            let mut row: Vec<Scalar> = Vec::with_capacity((divisions) as usize);
            for x in 0..divisions {
                let img_x = x * (TEXTURE_SIZE - 1) / (divisions - 1);
                let img_y = y * (TEXTURE_SIZE - 1) / (divisions - 1);

                // NOTE: This index is reversed from the mesh generation
                // has to do with parry's heightfield format
                let index = (img_x * TEXTURE_SIZE + img_y) * pixel_size;

                if index + 3 < image.data.len() {
                    let pixel_data = &image.data[index..(index + pixel_size)];
                    let height = pixel_data[0] as f32 / 255.0;
                    row.push(height * self.height_scale);
                }
            }
            heights.push(row);
        }
        //heights.reverse();
        Collider::heightfield(heights, scale)
    }
}
