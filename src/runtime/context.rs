use crate::vfs::Vfs;
use indexmap::IndexMap;
use shellframe::Context as ShellContext;

pub struct NashState {
    pub vfs: Vfs,
    pub allowed_bins: IndexMap<String, String>,
    pub history: Vec<String>,
}

pub type Context = ShellContext<NashState>;

impl NashState {
    pub fn new(vfs: Vfs, allowed_bins: IndexMap<String, String>) -> Self {
        Self {
            vfs,
            allowed_bins,
            history: Vec::new(),
        }
    }
}
