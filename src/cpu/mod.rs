mod cop0;
mod cpu;
mod instruction;
mod interconnect;

pub use self::cpu::CPU;
use self::cop0::cop0::Cop0;
use self::instruction::Instruction;
use self::interconnect::Interconnect;
