pub use model_merger_app_core::{
    GroupLog, GroupSession as GroupUiState, WorkspaceState as NativeAppState,
};

pub fn slot_columns(available_width: f32) -> usize {
    if available_width >= 712.0 {
        5
    } else if available_width >= 430.0 {
        3
    } else {
        2
    }
}
