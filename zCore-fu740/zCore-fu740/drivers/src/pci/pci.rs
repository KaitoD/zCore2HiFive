use core::convert::TryInto;
use core::ops::{BitAnd, BitOr, Not};

use bitflags::bitflags;
use spin::Mutex;

use crate::io::{Io, Mmio, ReadOnly};
use crate::scheme::{impl_event_scheme, Scheme, PciScheme};
use crate::utils::EventListener;
use crate::DeviceResult;

bitflags! {
    /// Interrupt enable flags
    struct IntEnFlags: u8 {
        const RECEIVED = 1;
        const SENT = 1 << 1;
        const ERRORED = 1 << 2;
        const STATUS_CHANGE = 1 << 3;
        // 4 to 7 are unused
    }
}

bitflags! {
    /// Line status flags
    struct LineStsFlags: u8 {
        const INPUT_FULL = 1;
        // 1 to 4 unknown
        const OUTPUT_EMPTY = 1 << 5;
        // 6 and 7 unknown
    }
}

#[repr(C)]
struct PciInner<T: Io> {
    /// Data register, read to receive, write to send
    data: T,
    /// Interrupt enable
    int_en: T,
    /// FIFO control
    fifo_ctrl: T,
    /// Line control
    line_ctrl: T,
    /// Modem control
    modem_ctrl: T,
    /// Line status
    line_sts: ReadOnly<T>,
    /// Modem status
    modem_sts: ReadOnly<T>,
}

impl<T: Io> PciInner<T>
where
    T::Value: From<u8> + TryInto<u8>,
{
    fn init(&mut self) {
        // Disable interrupts
        self.int_en.write(0x00.into());

        // Enable DLAB
        self.line_ctrl.write(0x80.into());

        // Set maximum speed to 38400 bps by configuring DLL and DLM
        self.data.write(0x03.into());
        self.int_en.write(0x00.into());

        // Disable DLAB and set data word length to 8 bits
        self.line_ctrl.write(0x03.into());

        // Enable FIFO, clear TX/RX queues and
        // set interrupt watermark at 14 bytes
        self.fifo_ctrl.write(0xC7.into());

        // Mark data terminal ready, signal request to send
        // and enable auxilliary output #2 (used as interrupt line for CPU)
        self.modem_ctrl.write(0x0B.into());

        // Enable interrupts
        self.int_en.write(0x01.into());
    }

    fn line_sts(&self) -> LineStsFlags {
        LineStsFlags::from_bits_truncate(
            (self.line_sts.read() & 0xFF.into()).try_into().unwrap_or(0),
        )
    }

    fn try_recv(&mut self) -> DeviceResult<Option<u8>> {
        if self.line_sts().contains(LineStsFlags::INPUT_FULL) {
            Ok(Some(
                (self.data.read() & 0xFF.into()).try_into().unwrap_or(0),
            ))
        } else {
            Ok(None)
        }
    }

    fn send(&mut self, ch: u8) -> DeviceResult {
        while !self.line_sts().contains(LineStsFlags::OUTPUT_EMPTY) {}
        self.data.write(ch.into());
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> DeviceResult {
        for b in s.bytes() {
            match b {
                b'\n' => {
                    self.send(b'\r')?;
                    self.send(b'\n')?;
                }
                _ => {
                    self.send(b)?;
                }
            }
        }
        Ok(())
    }
}

pub struct PciMmio<V: 'static>
where
    V: Copy + BitAnd<Output = V> + BitOr<Output = V> + Not<Output = V>,
{
    inner: Mutex<&'static mut PciInner<Mmio<V>>>,
    listener: EventListener,
}

impl_event_scheme!(PciMmio<V>
where
    V: Copy
        + BitAnd<Output = V>
        + BitOr<Output = V>
        + Not<Output = V>
        + From<u8>
        + TryInto<u8>
        + Send
);

impl<V> Scheme for PciMmio<V>
where
    V: Copy + BitAnd<Output = V> + BitOr<Output = V> + Not<Output = V> + Send,
{
    fn name(&self) -> &str {
        "pci-mmio"
    }

    fn handle_irq(&self, _irq_num: usize) {
        self.listener.trigger(());
    }
}

impl<V> PciScheme for PciMmio<V>
where
    V: Copy
        + BitAnd<Output = V>
        + BitOr<Output = V>
        + Not<Output = V>
        + From<u8>
        + TryInto<u8>
        + Send,
{
    fn try_recv(&self) -> DeviceResult<Option<u8>> {
        self.inner.lock().try_recv()
    }

    fn send(&self, ch: u8) -> DeviceResult {
        self.inner.lock().send(ch)
    }

    fn write_str(&self, s: &str) -> DeviceResult {
        self.inner.lock().write_str(s)
    }
}

impl<V> PciMmio<V>
where
    V: Copy
        + BitAnd<Output = V>
        + BitOr<Output = V>
        + Not<Output = V>
        + From<u8>
        + TryInto<u8>
        + Send,
{
    unsafe fn new_common(base: usize) -> Self {
        let Pci: &mut PciInner<Mmio<V>> = Mmio::<V>::from_base_as(base);
        Pci.init();
        Self {
            inner: Mutex::new(Pci),
            listener: EventListener::new(),
        }
    }
}

impl PciMmio<u8> {
    /// # Safety
    ///
    /// This function is unsafe because `base_addr` may be an arbitrary address.
    pub unsafe fn new(base: usize) -> Self {
        Self::new_common(base)
    }
}

impl PciMmio<u32> {
    /// # Safety
    ///
    /// This function is unsafe because `base_addr` may be an arbitrary address.
    pub unsafe fn new(base: usize) -> Self {
        Self::new_common(base)
    }
}
