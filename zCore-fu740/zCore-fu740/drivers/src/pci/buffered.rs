use alloc::{boxed::Box, collections::VecDeque, string::String, sync::Arc};

use spin::Mutex;

use crate::scheme::{impl_event_scheme, Scheme, PciScheme};
use crate::utils::EventListener;
use crate::DeviceResult;

const BUF_CAPACITY: usize = 4096;

pub struct BufferedPci {
    inner: Arc<dyn PciScheme>,
    buf: Mutex<VecDeque<u8>>,
    listener: EventListener,
    name: String,
}

impl_event_scheme!(BufferedPci);

impl BufferedPci {
    pub fn new(pci: Arc<dyn PciScheme>) -> Arc<Self> {
        let ret = Arc::new(Self {
            inner: pci.clone(),
            name: alloc::format!("{}-buffered", pci.name()),
            buf: Mutex::new(VecDeque::with_capacity(BUF_CAPACITY)),
            listener: EventListener::new(),
        });
        let cloned = ret.clone();
        pci.subscribe(Box::new(move |_| cloned.handle_irq(0)), false);
        ret
    }
}

impl Scheme for BufferedPci {
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn handle_irq(&self, _unused: usize) {
        while let Some(c) = self.inner.try_recv().unwrap_or(None) {
            let mut buf = self.buf.lock();
            if buf.len() < BUF_CAPACITY {
                //let c = if c == b'\r' { b'\n' } else { c };
                buf.push_back(c);
            }
        }
        if self.buf.lock().len() > 0 {
            self.listener.trigger(());
        }
    }
}

impl PciScheme for BufferedPci {
    fn try_recv(&self) -> DeviceResult<Option<u8>> {
        Ok(self.buf.lock().pop_front())
    }
    fn send(&self, ch: u8) -> DeviceResult {
        self.inner.send(ch)
    }
    fn write_str(&self, s: &str) -> DeviceResult {
        self.inner.write_str(s)
    }
}
