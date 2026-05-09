pub mod dispersion;
pub mod engine;
pub mod fractional_delay;
pub mod hold_stack;
pub mod karplus_strong;
pub mod lfo;
pub mod loss_filter;
pub mod modal_body;
pub mod note_allocator;
pub mod params;
pub mod rng;
pub mod smoothing;
pub mod soft_clip;
pub mod sustain_state;
pub mod traits;
pub mod voice;
pub mod voice_pool;

pub use dispersion::{compute_dispersion_a1, DispersionStage, DISPERSION_STAGES};
