//! RP2A03 NES APU (Audio Processing Unit) Hardware Layer
//!
//! ファミコン実機（Ricoh RP2A03 APU）のデジタル論理回路・変調回路・アナログ回路群を集約するモジュールです。

pub mod core;
pub mod dmc;
pub mod envelope;
pub mod filter;
pub mod frame_counter;
pub mod length_counter;
pub mod linear_counter;
pub mod noise;
pub mod pulse;
pub mod sweep;
pub mod triangle;

pub use core::{register_to_channel, NesApuCore, RegisterWriteSource, NTSC_CPU_CLOCK};
pub use dmc::{DmcChannel, DMC_NTSC_RATE_TABLE};
pub use envelope::EnvelopeGenerator;
pub use filter::{AnalogFilterChain, HighPassFilter, LowPassFilter, NonlinearMixer};
pub use frame_counter::{FrameClockAction, FrameCounter};
pub use length_counter::{LengthCounter, LENGTH_COUNTER_TABLE};
pub use linear_counter::LinearCounter;
pub use noise::{NoiseChannel, NOISE_PERIOD_TABLE_NTSC};
pub use pulse::{PulseChannel, PULSE_DUTY_TABLE};
pub use sweep::SweepUnit;
pub use triangle::{TriangleChannel, TRIANGLE_SEQUENCE};
