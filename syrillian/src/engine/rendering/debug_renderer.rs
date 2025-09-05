use std::sync::RwLock;

#[cfg(debug_assertions)]
static DEBUG_RENDERER: RwLock<DebugRenderer> = RwLock::new(DebugRenderer::default_const());

#[derive(Debug, Clone)]
pub struct DebugRenderer {
    pub mesh_edges: bool,
    pub vertex_normals: bool,
    pub rays: bool,
    pub colliders_edges: bool,
    pub text_geometry: bool,
    pub light: bool,
}

impl Default for DebugRenderer {
    fn default() -> Self {
        Self::default_const()
    }
}

impl DebugRenderer {
    const fn default_const() -> Self {
        const DEBUG_BUILD: bool = cfg!(debug_assertions);

        DebugRenderer {
            mesh_edges: DEBUG_BUILD,
            colliders_edges: false,
            vertex_normals: false,
            rays: DEBUG_BUILD,
            text_geometry: DEBUG_BUILD,
            light: DEBUG_BUILD,
        }
    }

    pub fn next_mode() -> u32 {
        let mut inner = DEBUG_RENDERER.write().unwrap_or_else(|e| {
            log::error!(
                "Failed to get a writable inner to DEBUG_RENDERER in next_mode : {}",
                e
            );
            std::process::exit(1);
        });
        if inner.mesh_edges && !inner.vertex_normals {
            inner.vertex_normals = true;
            1
        } else if inner.mesh_edges {
            inner.mesh_edges = false;
            inner.vertex_normals = false;
            inner.colliders_edges = true;
            2
        } else if inner.colliders_edges {
            *inner = DebugRenderer {
                mesh_edges: false,
                colliders_edges: false,
                vertex_normals: false,
                rays: false,
                text_geometry: false,
                light: false,
            };
            3
        } else {
            *inner = DebugRenderer::default();
            0
        }
    }

    // TODO: Turn these into a macro
    pub fn mesh_edges() -> bool {
        let inner = DEBUG_RENDERER.read().unwrap_or_else(|e| {
            log::error!(
                "Failed to get a readable inner to DEBUG_RENDERER in mesh_edges : {}",
                e
            );
            std::process::exit(1);
        });
        inner.mesh_edges
    }

    pub fn collider_mesh() -> bool {
        let inner = DEBUG_RENDERER.read().unwrap_or_else(|e| {
            log::error!(
                "Failed to get a readable inner to DEBUG_RENDERER in collider_mesh : {}",
                e
            );
            std::process::exit(1);
        });
        inner.colliders_edges
    }

    pub fn mesh_vertex_normals() -> bool {
        let inner = DEBUG_RENDERER.read().unwrap_or_else(|e| {
            log::error!(
                "Failed to get a readable inner to DEBUG_RENDERER in mesh_vertex_normals : {}",
                e
            );
            std::process::exit(1);
        });
        inner.vertex_normals
    }

    pub fn physics_rays() -> bool {
        let inner = DEBUG_RENDERER.read().unwrap_or_else(|e| {
            log::error!(
                "Failed to get a readable inner to DEBUG_RENDERER in physics_rays : {}",
                e
            );
            std::process::exit(1);
        });
        inner.rays
    }

    pub fn text_geometry() -> bool {
        let inner = DEBUG_RENDERER.read().unwrap_or_else(|e| {
            log::error!(
                "Failed to get a readable inner to DEBUG_RENDERER in text_geometry : {}",
                e
            );
            std::process::exit(1);
        });
        inner.text_geometry
    }

    pub fn light() -> bool {
        let inner = DEBUG_RENDERER.read().unwrap_or_else(|e| {
            log::error!(
                "Failed to get a readable inner to DEBUG_RENDERER in light : {}",
                e
            );
            std::process::exit(1);
        });
        inner.light
    }

    pub fn off() {
        let mut inner = DEBUG_RENDERER.write().unwrap_or_else(|e| {
            log::error!(
                "Failed to get a writable inner to DEBUG_RENDERER in off : {}",
                e
            );
            std::process::exit(1);
        });
        *inner = DebugRenderer {
            mesh_edges: false,
            vertex_normals: false,
            rays: false,
            colliders_edges: false,
            text_geometry: false,
            light: false,
        }
    }
}
