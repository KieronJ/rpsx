#[derive(Default)]
pub struct Status {
	coprocessor_enable: [bool; 4],
	reverse_endianness: bool,
	boot_exception_vector: bool,
	tlb_shutdown: bool,
	cache_parity_error: bool,
	last_load_result: bool,
	cache_parity_zero: bool,
	swapped_cache: bool,
	isolate_cache: bool,
	interrupt_mask: u8,
	interrupt_enable_kernel_user_mode: InterruptEnableKernelUserMode
}

impl Status {
	pub fn get_value(&self) -> u32 {
		let mut value  = (self.coprocessor_enable[0]  as u32) << 31;
				value |= (self.coprocessor_enable[1]  as u32) << 30;
				value |= (self.coprocessor_enable[2]  as u32) << 29;
				value |= (self.coprocessor_enable[3]  as u32) << 28;
				value |= (self.reverse_endianness     as u32) << 25;
				value |= (self.boot_exception_vector  as u32) << 22;
				value |= (self.tlb_shutdown           as u32) << 21;
				value |= (self.cache_parity_error     as u32) << 20;
				value |= (self.last_load_result       as u32) << 19;
				value |= (self.cache_parity_zero      as u32) << 18;
				value |= (self.swapped_cache          as u32) << 17;
				value |= (self.isolate_cache          as u32) << 16;
				value |= (self.interrupt_mask         as u32) <<  8;
				value |  (self.interrupt_enable_kernel_user_mode.get_value() as u32)
	}

	pub fn set_value(&mut self, data: u32) {
		self.coprocessor_enable[0]  = (data & (1 << 31)) != 0;
		self.coprocessor_enable[1]  = (data & (1 << 30)) != 0;
		self.coprocessor_enable[2]  = (data & (1 << 29)) != 0;
		self.coprocessor_enable[3]  = (data & (1 << 28)) != 0;
		self.reverse_endianness     = (data & (1 << 25)) != 0;
		self.boot_exception_vector  = (data & (1 << 22)) != 0;
		self.tlb_shutdown           = (data & (1 << 21)) != 0;
		self.cache_parity_error     = (data & (1 << 20)) != 0;
		self.last_load_result       = (data & (1 << 19)) != 0;
		self.cache_parity_zero      = (data & (1 << 18)) != 0;
		self.swapped_cache          = (data & (1 << 17)) != 0;
		self.isolate_cache          = (data & (1 << 16)) != 0;
		self.interrupt_mask         = ((data >>  8) & 0xff) as u8;
		self.interrupt_enable_kernel_user_mode.set_value(data);
	}

	pub fn isolate_cache(&self) -> bool {
		self.isolate_cache
	}
}

#[derive(Default)]
struct InterruptEnableKernelUserMode {
	old_kernel_user_mode: bool,
	old_interrupt_enable: bool,
	prev_kernel_user_mode: bool,
	prev_interrupt_enable: bool,
	curr_kernel_user_mode: bool,
	curr_interrupt_enable: bool
}

impl InterruptEnableKernelUserMode {
	pub fn get_value(&self) -> u8 {
		let mut value = 0u8;
		value |= (self.old_kernel_user_mode  as u8) << 5;
		value |= (self.old_interrupt_enable  as u8) << 4;
		value |= (self.prev_kernel_user_mode as u8) << 3;
		value |= (self.prev_interrupt_enable as u8) << 2;
		value |= (self.curr_kernel_user_mode as u8) << 1;
		value |  (self.curr_interrupt_enable as u8)
	}

	pub fn set_value(&mut self, data: u32) {
		self.old_kernel_user_mode   = (data & (1 << 5)) != 0;
		self.old_interrupt_enable   = (data & (1 << 4)) != 0;
		self.prev_kernel_user_mode  = (data & (1 << 3)) != 0;
		self.prev_interrupt_enable  = (data & (1 << 2)) != 0;
		self.curr_kernel_user_mode  = (data & (1 << 1)) != 0;
		self.curr_interrupt_enable  = (data & (1 << 0)) != 0;
	}

	pub fn enter_exception(&mut self) {
		self.old_kernel_user_mode  = self.prev_kernel_user_mode;
		self.old_interrupt_enable  = self.prev_interrupt_enable;
		self.prev_kernel_user_mode = self.curr_kernel_user_mode;
		self.prev_interrupt_enable = self.curr_interrupt_enable;
		self.curr_kernel_user_mode = false;
		self.curr_interrupt_enable = false;
	}

	pub fn leave_exception(&mut self) {
		self.curr_kernel_user_mode = self.prev_kernel_user_mode;
		self.curr_interrupt_enable = self.prev_interrupt_enable;
		self.prev_kernel_user_mode = self.old_kernel_user_mode;
		self.prev_interrupt_enable = self.old_interrupt_enable;

	}
}