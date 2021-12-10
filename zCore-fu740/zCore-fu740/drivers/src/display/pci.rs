//! PCIe Graphics Output Protocol

use crate::prelude::{DisplayInfo, FrameBuffer};
use crate::scheme::{DisplayScheme, Scheme};

pub struct PciDisplay {
    info: DisplayInfo,
}

impl PciDisplay {
    pub fn new(info: DisplayInfo) -> Self {
        Self { info }
    }
}

impl Scheme for PciDisplay {
    fn name(&self) -> &str {
        "pci-display"
    }
}

impl DisplayScheme for PciDisplay {
    #[inline]
    fn info(&self) -> DisplayInfo {
        self.info
    }

    #[inline]
    fn fb(&self) -> FrameBuffer {
        unsafe {
            FrameBuffer::from_raw_parts_mut(self.info.fb_base_vaddr as *mut u8, self.info.fb_size)
        }
    }
}
