#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceId(String);

impl SurfaceId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SurfaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceBackendKind {
    Native,
    Tmux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceEventKind {
    LaunchRequested,
    HudRequested,
    Attached,
    CleanedUp,
    RenderRequested,
    FallbackDirect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceEvent {
    pub surface_id: SurfaceId,
    pub backend: SurfaceBackendKind,
    pub kind: SurfaceEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderSurface {
    pub id: SurfaceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerSurface {
    pub id: SurfaceId,
    pub worker_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudSurface {
    pub id: SurfaceId,
    pub watch: bool,
}

pub trait SurfaceBackend {
    fn kind(&self) -> SurfaceBackendKind;
    fn leader_surface(&self) -> LeaderSurface;
    fn worker_surface(&self, worker_name: &str) -> WorkerSurface;
    fn hud_surface(&self, watch: bool) -> HudSurface;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBackend {
    runtime_session_id: String,
}

impl NativeBackend {
    #[must_use]
    pub fn new(runtime_session_id: impl Into<String>) -> Self {
        Self {
            runtime_session_id: runtime_session_id.into(),
        }
    }
}

impl SurfaceBackend for NativeBackend {
    fn kind(&self) -> SurfaceBackendKind {
        SurfaceBackendKind::Native
    }

    fn leader_surface(&self) -> LeaderSurface {
        LeaderSurface {
            id: SurfaceId::new(format!("native:{}:leader", self.runtime_session_id)),
        }
    }

    fn worker_surface(&self, worker_name: &str) -> WorkerSurface {
        WorkerSurface {
            id: SurfaceId::new(format!(
                "native:{}:worker:{worker_name}",
                self.runtime_session_id
            )),
            worker_name: worker_name.to_string(),
        }
    }

    fn hud_surface(&self, watch: bool) -> HudSurface {
        let mode = if watch { "watch" } else { "inline" };
        HudSurface {
            id: SurfaceId::new(format!("native:{}:hud:{mode}", self.runtime_session_id)),
            watch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxBackend {
    session_name: String,
}

impl TmuxBackend {
    #[must_use]
    pub fn new(session_name: impl Into<String>) -> Self {
        Self {
            session_name: session_name.into(),
        }
    }
}

impl SurfaceBackend for TmuxBackend {
    fn kind(&self) -> SurfaceBackendKind {
        SurfaceBackendKind::Tmux
    }

    fn leader_surface(&self) -> LeaderSurface {
        LeaderSurface {
            id: SurfaceId::new(format!("tmux:{}:leader", self.session_name)),
        }
    }

    fn worker_surface(&self, worker_name: &str) -> WorkerSurface {
        WorkerSurface {
            id: SurfaceId::new(format!("tmux:{}:worker:{worker_name}", self.session_name)),
            worker_name: worker_name.to_string(),
        }
    }

    fn hud_surface(&self, watch: bool) -> HudSurface {
        let mode = if watch { "watch" } else { "inline" };
        HudSurface {
            id: SurfaceId::new(format!("tmux:{}:hud:{mode}", self.session_name)),
            watch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRuntime<B> {
    backend: B,
}

impl<B> SurfaceRuntime<B>
where
    B: SurfaceBackend,
{
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self { backend }
    }

    #[must_use]
    pub fn backend_kind(&self) -> SurfaceBackendKind {
        self.backend.kind()
    }

    #[must_use]
    pub fn leader_surface(&self) -> LeaderSurface {
        self.backend.leader_surface()
    }

    #[must_use]
    pub fn worker_surface(&self, worker_name: &str) -> WorkerSurface {
        self.backend.worker_surface(worker_name)
    }

    #[must_use]
    pub fn hud_surface(&self, watch: bool) -> HudSurface {
        self.backend.hud_surface(watch)
    }

    #[must_use]
    pub fn event(&self, surface_id: SurfaceId, kind: SurfaceEventKind) -> SurfaceEvent {
        SurfaceEvent {
            surface_id,
            backend: self.backend_kind(),
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeBackend, SurfaceBackendKind, SurfaceEventKind, SurfaceRuntime, TmuxBackend};

    #[test]
    fn native_runtime_emits_native_surface_ids() {
        let runtime = SurfaceRuntime::new(NativeBackend::new("prompt-demo"));

        assert_eq!(runtime.backend_kind(), SurfaceBackendKind::Native);
        assert_eq!(
            runtime.leader_surface().id.as_str(),
            "native:prompt-demo:leader"
        );
        assert_eq!(
            runtime.worker_surface("worker-2").id.as_str(),
            "native:prompt-demo:worker:worker-2"
        );
        assert_eq!(
            runtime.hud_surface(true).id.as_str(),
            "native:prompt-demo:hud:watch"
        );
    }

    #[test]
    fn tmux_runtime_emits_tmux_surface_ids_and_events() {
        let runtime = SurfaceRuntime::new(TmuxBackend::new("omx-demo"));
        let hud = runtime.hud_surface(true);
        let event = runtime.event(hud.id.clone(), SurfaceEventKind::HudRequested);

        assert_eq!(runtime.backend_kind(), SurfaceBackendKind::Tmux);
        assert_eq!(hud.id.as_str(), "tmux:omx-demo:hud:watch");
        assert_eq!(event.backend, SurfaceBackendKind::Tmux);
        assert_eq!(event.kind, SurfaceEventKind::HudRequested);
        assert_eq!(event.surface_id.as_str(), "tmux:omx-demo:hud:watch");
    }
}
