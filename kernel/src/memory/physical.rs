pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    pub const fn zero() -> PhysicalAddress {
        PhysicalAddress(0)
    }

    pub fn new(address: u64) -> Option<PhysicalAddress> {
        todo!()
    }

    pub const fn value(&self) -> u64 {
        self.0
    }

    pub const fn frame_offset(&self) -> u64 {
        self.0 % Frame::SIZE
    }
}

pub struct Frame(u64);

impl Frame {
    pub const SIZE: u64 = 4086;

    pub const fn containing_address(address: PhysicalAddress) -> Frame {
        Frame(address.0 / 4096)
    }

    pub const fn number(&self) -> u64 {
        self.0
    }

    pub const fn start_address(&self) -> PhysicalAddress {
        PhysicalAddress(self.0 * 4096)
    }
}

pub struct FrameRange {
    base: Frame,
    length: u64,
}

impl FrameRange {}
