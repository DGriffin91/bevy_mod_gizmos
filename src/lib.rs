use std::mem;

use bevy::{
    asset::load_internal_asset,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_phase::AddRenderCommand,
        render_resource::{PrimitiveTopology, SpecializedMeshPipelines},
        Extract, RenderApp, RenderSet,
    },
};

#[cfg(feature = "bevy_pbr")]
use bevy::pbr::MeshUniform;
#[cfg(feature = "bevy_sprite")]
use bevy::sprite::{Mesh2dHandle, Mesh2dUniform};
pub mod gizmos;

#[cfg(feature = "bevy_sprite")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

use crate::gizmos::GizmoStorage;

/// The `bevy_gizmos` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{gizmos::Gizmos, GizmoConfig};
}

const LINE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, LINE_SHADER_HANDLE, "lines.wgsl", Shader::from_wgsl);

        app.init_resource::<MeshHandles>()
            .init_resource::<GizmoConfig>()
            .init_resource::<GizmoStorage>()
            .add_system(update_gizmo_meshes.in_base_set(CoreSet::Last));

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app.add_system(extract_gizmo_data.in_schedule(ExtractSchedule));

        #[cfg(feature = "bevy_sprite")]
        {
            use bevy::core_pipeline::core_2d::Transparent2d;
            use pipeline_2d::*;

            render_app
                .add_render_command::<Transparent2d, DrawGizmoLines>()
                .init_resource::<GizmoLinePipeline>()
                .init_resource::<SpecializedMeshPipelines<GizmoLinePipeline>>()
                .add_system(queue_gizmos_2d.in_set(RenderSet::Queue));
        }

        #[cfg(feature = "bevy_pbr")]
        {
            use bevy::core_pipeline::core_3d::Opaque3d;
            use pipeline_3d::*;

            render_app
                .add_render_command::<Opaque3d, DrawGizmoLines>()
                .init_resource::<GizmoPipeline>()
                .init_resource::<SpecializedMeshPipelines<GizmoPipeline>>()
                .add_system(queue_gizmos_3d.in_set(RenderSet::Queue));
        }
    }
}

#[derive(Resource, Clone, Copy)]
pub struct GizmoConfig {
    /// Set to `false` to stop drawing gizmos.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Draw gizmos on top of everything else, ignoring depth.
    ///
    /// This setting only affects 3D. In 2D, gizmos are always drawn on top.
    ///
    /// Defaults to `false`.
    pub on_top: bool,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            on_top: false,
        }
    }
}

#[derive(Resource)]
struct MeshHandles {
    list: Option<Handle<Mesh>>,
    strip: Option<Handle<Mesh>>,
}

impl FromWorld for MeshHandles {
    fn from_world(_world: &mut World) -> Self {
        MeshHandles {
            list: None,
            strip: None,
        }
    }
}

#[derive(Component)]
struct GizmoMesh;

fn update_gizmo_meshes(
    mut meshes: ResMut<Assets<Mesh>>,
    mut handles: ResMut<MeshHandles>,
    mut storage: ResMut<GizmoStorage>,
) {
    if storage.list_positions.is_empty() {
        handles.list = None;
    } else if let Some(handle) = handles.list.as_ref() {
        let list_mesh = meshes.get_mut(handle).unwrap();

        let positions = mem::take(&mut storage.list_positions);
        list_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let colors = mem::take(&mut storage.list_colors);
        list_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    } else {
        let mut list_mesh = Mesh::new(PrimitiveTopology::LineList);

        let positions = mem::take(&mut storage.list_positions);
        list_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let colors = mem::take(&mut storage.list_colors);
        list_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        handles.list = Some(meshes.add(list_mesh));
    }

    if storage.strip_positions.is_empty() {
        handles.strip = None;
    } else if let Some(handle) = handles.strip.as_ref() {
        let strip_mesh = meshes.get_mut(handle).unwrap();

        let positions = mem::take(&mut storage.strip_positions);
        strip_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let colors = mem::take(&mut storage.strip_colors);
        strip_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    } else {
        let mut strip_mesh = Mesh::new(PrimitiveTopology::LineStrip);

        let positions = mem::take(&mut storage.strip_positions);
        strip_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        let colors = mem::take(&mut storage.strip_colors);
        strip_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);

        handles.strip = Some(meshes.add(strip_mesh));
    }
}

fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<MeshHandles>>,
    config: Extract<Res<GizmoConfig>>,
) {
    if config.is_changed() {
        commands.insert_resource(**config);
    }

    if !config.enabled {
        return;
    }

    let transform = Mat4::IDENTITY;
    let inverse_transpose_model = transform.inverse().transpose();
    commands.spawn_batch(
        [handles.list.clone(), handles.strip.clone()]
            .into_iter()
            .flatten()
            .map(move |handle| {
                (
                    GizmoMesh,
                    #[cfg(feature = "bevy_pbr")]
                    (
                        handle.clone_weak(),
                        MeshUniform {
                            flags: 0,
                            transform,
                            inverse_transpose_model,
                        },
                    ),
                    #[cfg(feature = "bevy_sprite")]
                    (
                        Mesh2dHandle(handle),
                        Mesh2dUniform {
                            flags: 0,
                            transform,
                            inverse_transpose_model,
                        },
                    ),
                )
            }),
    );
}
