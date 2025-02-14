use std::sync::Arc;

use flax::{
    entity_ids,
    fetch::{Modified, TransformFetch},
    filter::{All, With},
    CommandBuffer, Component, EntityIds, Fetch, FetchExt, Mutable, Query,
};
use glam::{vec2, vec3, Mat4, Quat, Vec2};
use image::{DynamicImage, ImageBuffer};
use wgpu::{BindGroup, BindGroupLayout, SamplerDescriptor, ShaderStages, TextureFormat};

use crate::{
    assets::{map::HandleMap, Handle},
    components::{filled_rect, rect, screen_position, Rect},
    shapes::FilledRect,
    Frame,
};

use super::{
    components::{draw_cmd, mesh_handle, model_matrix},
    graphics::{
        shader::ShaderDesc, texture::Texture, BindGroupBuilder, BindGroupLayoutBuilder, Shader,
        Vertex, VertexDesc,
    },
    mesh_buffer::MeshHandle,
    renderer::RendererContext,
    shape_renderer::DrawCommand,
    Gpu,
};

#[derive(Fetch)]
struct RectQuery {
    rect: Component<Rect>,
    pos: Component<Vec2>,
    model: Mutable<Mat4>,
}

impl RectQuery {
    fn new() -> Self {
        Self {
            rect: rect(),
            pos: screen_position(),
            model: model_matrix().as_mut(),
        }
    }
}

pub struct RectRenderer {
    white_image: Handle<DynamicImage>,

    layout: BindGroupLayout,
    sampler: wgpu::Sampler,

    rect_query: Query<(
        EntityIds,
        <Component<FilledRect> as TransformFetch<Modified>>::Output,
    )>,

    object_query: Query<RectQuery, (All, With)>,

    bind_groups: HandleMap<DynamicImage, Handle<BindGroup>>,

    mesh: Arc<MeshHandle>,

    shader: Handle<Shader>,
}

impl RectRenderer {
    pub fn new(
        ctx: &mut RendererContext,
        frame: &Frame,
        color_format: TextureFormat,
        object_bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let layout = BindGroupLayoutBuilder::new("RectRenderer::layout")
            .bind_sampler(ShaderStages::FRAGMENT)
            .bind_texture(ShaderStages::FRAGMENT)
            .build(&ctx.gpu);

        let white_image = frame
            .assets
            .insert(DynamicImage::ImageRgba8(ImageBuffer::from_pixel(
                256,
                256,
                image::Rgba([255, 255, 255, 255]),
            )));

        let sampler = ctx.gpu.device.create_sampler(&SamplerDescriptor {
            label: Some("ShapeRenderer::sampler"),
            anisotropy_clamp: 16,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let vertices = [
            Vertex::new(vec3(0.0, 0.0, 0.0), vec2(0.0, 0.0)),
            Vertex::new(vec3(1.0, 0.0, 0.0), vec2(1.0, 0.0)),
            Vertex::new(vec3(1.0, 1.0, 0.0), vec2(1.0, 1.0)),
            Vertex::new(vec3(0.0, 1.0, 0.0), vec2(0.0, 1.0)),
        ];

        let indices = [0, 1, 2, 2, 3, 0];

        let mesh = Arc::new(ctx.mesh_buffer.insert(&ctx.gpu, &vertices, &indices));

        let shader = frame.assets.insert(Shader::new(
            &ctx.gpu,
            &ShaderDesc {
                label: "ShapeRenderer::shader",
                source: include_str!("../../assets/shaders/solid.wgsl"),
                format: color_format,
                vertex_layouts: &[Vertex::layout()],
                layouts: &[&ctx.globals_layout, &object_bind_group_layout, &layout],
            },
        ));

        Self {
            white_image,
            layout,
            sampler,
            rect_query: Query::new((entity_ids(), filled_rect().modified())),
            object_query: Query::new(RectQuery::new()).with(filled_rect()),
            bind_groups: HandleMap::new(),
            mesh,
            shader,
        }
    }

    pub fn build_commands(&mut self, gpu: &Gpu, frame: &mut Frame) {
        let mut cmd = CommandBuffer::new();
        self.rect_query
            .borrow(&frame.world)
            .iter()
            .for_each(|(id, rect)| {
                let image = rect.fill_image.as_ref().unwrap_or(&self.white_image);

                let bind_group = self.bind_groups.entry(&image.clone()).or_insert_with(|| {
                    let texture = Texture::from_image(gpu, image);

                    let bind_group = BindGroupBuilder::new("ShapeRenderer::textured_bind_group")
                        .bind_sampler(&self.sampler)
                        .bind_texture(&texture.view(&Default::default()))
                        .build(gpu, &self.layout);

                    frame.assets.insert(bind_group)
                });

                cmd.set(id, mesh_handle(), self.mesh.clone()).set(
                    id,
                    draw_cmd(),
                    DrawCommand {
                        bind_group: bind_group.clone(),
                        shader: self.shader.clone(),
                        index_count: 6,
                        vertex_offset: 0,
                    },
                );
            });

        cmd.apply(&mut frame.world).unwrap();
    }

    pub fn update(&mut self, _: &Gpu, frame: &Frame) {
        self.object_query
            .borrow(&frame.world)
            .iter()
            .for_each(|item| {
                let rect = item.rect.translate(*item.pos).align_to_grid();

                *item.model = Mat4::from_scale_rotation_translation(
                    rect.size().extend(1.0),
                    Quat::IDENTITY,
                    rect.pos().extend(0.1),
                );
            })
    }
}
