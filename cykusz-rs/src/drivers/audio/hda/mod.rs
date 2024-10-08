mod cmd;
mod node;
mod reg;
mod stream;

use crate::arch::mm::{PhysAddr, PAGE_SIZE};
use crate::drivers::audio::hda::reg::verb;
use crate::drivers::audio::hda::reg::verb::{
    ConfigurationDefaultReg, GetParameterAudioWidgetCapReg, GetParameterInputAmplifierCap,
    GetParameterNodeCount, GetParameterOutputAmplifierCap, GetParameterPinCap, GetPinSense,
    NodeCommand, SetAmplifierGainMute, SetChannelStreamID, SetConverterFormat, SetEAPDBTLEnable,
    SetPinWidgetControl, SetPowerState,
};
use crate::drivers::pci::{register_pci_device, PciDeviceHandle, PciHeader};
use crate::kernel::device::dev_t::DevId;
use crate::kernel::device::{register_device, Device};
use crate::kernel::fs::inode::INode;
use crate::kernel::fs::pcache::{MMapPage, MMapPageStruct, MappedAccess, PageDirectItemStruct};
use crate::kernel::fs::poll::PollTable;
use crate::kernel::mm::virt::PageFlags;
use crate::kernel::mm::{allocate_order, map_to_flags, VirtAddr};
use crate::kernel::sync::{LockApi, Spin};
use crate::kernel::utils::types::Align;
use crate::kernel::utils::wait_queue::WaitQueue;
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use bit_field::BitField;
use spin::Once;
use syscall_defs::poll::PollEventFlags;
use tock_registers::interfaces::Writeable;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct Address {
    codec: u32,
    node: u32,
}

impl Address {
    pub fn new(codec: u32, node: u32) -> Address {
        Address { codec, node }
    }
    pub fn codec(&self) -> u32 {
        self.codec
    }

    pub fn node(&self) -> u32 {
        self.node
    }
}

impl From<(u32, u32)> for Address {
    fn from(value: (u32, u32)) -> Self {
        Address::new(value.0, value.1)
    }
}

fn hda_int_handler() {
    device().handle_interrupt();
}
fn sh_hda_int_handler() -> bool {
    device().handle_interrupt()
}

struct IntelHdaData {
    reg: &'static mut reg::Regs,
    cmd: cmd::Command,
    nodes: hashbrown::HashMap<Address, node::Node>,
    outputs: Vec<Address>,
    inputs: Vec<Address>,
    output_pins: Vec<Address>,
    input_pins: Vec<Address>,
    beep_addr: Option<Address>,
    streams: Vec<stream::Stream>,
    output_streams: Vec<stream::OutputStream>,
    posbuf: PhysAddr,
}

impl IntelHdaData {
    pub fn new(base_addr: VirtAddr) -> IntelHdaData {
        IntelHdaData {
            reg: unsafe { base_addr.read_mut::<reg::Regs>() },
            cmd: cmd::Command::new(base_addr, false),
            nodes: hashbrown::HashMap::new(),
            outputs: Vec::new(),
            inputs: Vec::new(),
            output_pins: Vec::new(),
            input_pins: Vec::new(),
            beep_addr: None,
            streams: Vec::new(),
            output_streams: Vec::new(),
            posbuf: PhysAddr(0),
        }
    }

    #[allow(unused)]
    fn in_stream(&self, idx: usize) -> Option<stream::Stream> {
        let num_input_streams = self.reg.gcap.iss() as usize;

        if idx >= num_input_streams {
            return None;
        }

        Some(self.streams[idx])
    }

    fn out_stream(&self, idx: usize) -> Option<stream::Stream> {
        let num_output_streams = self.reg.gcap.oss() as usize;

        if idx >= num_output_streams {
            return None;
        }

        let num_input_streams = self.reg.gcap.iss() as usize;

        Some(self.streams[num_input_streams + idx])
    }

    #[allow(unused)]
    fn bss_stream(&self, idx: usize) -> Option<stream::Stream> {
        let num_bss_streams = self.reg.gcap.bss() as usize;

        if idx >= num_bss_streams {
            return None;
        }

        let num_input_streams = self.reg.gcap.iss() as usize;
        let num_output_streams = self.reg.gcap.oss() as usize;

        Some(self.streams[num_input_streams + num_output_streams + idx])
    }

    fn handle_interrupt(&mut self, wq: &WaitQueue) -> bool {
        let ints = self.reg.intsts.get_local();

        dbgln!(audio, "interrupt: {:#b}", ints.get());

        if ints.is_gis() {
            if ints.is_cis() {
                // handle controller interrupt
            }

            let sis = ints.sis();

            let iss = self.reg.gcap.iss() as usize;
            let oss = self.reg.gcap.oss() as usize;
            let bss = self.reg.gcap.bss() as usize;

            for i in 0..iss {
                if sis.get_bit(i) {
                    let s = self.in_stream(i).unwrap();
                    s.clear_interrupts();
                }
            }
            for i in 0..oss {
                if sis.get_bit(iss + i) {
                    let s = self.out_stream(i).unwrap();

                    dbgln!(
                        audio,
                        "link pos: {} (b: {}) {:#x}",
                        s.link_position(),
                        s.dpl_link_position(),
                        s.status().get()
                    );
                    s.clear_interrupts();

                    wq.notify_one();
                }
            }
            for i in 0..bss {
                if sis.get_bit(iss + oss + i) {
                    let s = self.bss_stream(i).unwrap();
                    s.clear_interrupts();
                }
            }
        }

        ints.get() != 0
    }

    fn setup_interrupts(&mut self, pci: &PciHeader) {
        let mut is_msi = true;

        if let Some(int) = pci.enable_msi_interrupt(hda_int_handler).or_else(|| {
            is_msi = false;
            pci.enable_pci_interrupt(sh_hda_int_handler)
        }) {
            dbgln!(
                audio,
                "[ HDA ] Using {} interrupt: {}",
                if is_msi { "MSI" } else { "PCI" },
                int
            );
        }

        self.reg.intctl.set_is_gie(true);
        self.reg.intctl.set_is_cie(true);
        self.reg.intctl.set_sie(0b1111_1111_1111);
    }

    fn enumerate_nodes(&mut self, codec: u32, range: core::ops::Range<u32>) {
        for node in range {
            let address = Address::new(codec, node);

            dbgln!(audio, "===========");
            let node = node::Node::new(address, true, &mut self.cmd);

            match node.capabilities().typ() {
                Some(GetParameterAudioWidgetCapReg::TYPE::Value::AudioOutput) => {
                    dbgln!(audio, "Output: {:?}", address);
                    self.outputs.push(address);
                }
                Some(GetParameterAudioWidgetCapReg::TYPE::Value::AudioInput) => {
                    dbgln!(audio, "Input: {:?}", address);
                    self.inputs.push(address);
                }
                Some(GetParameterAudioWidgetCapReg::TYPE::Value::BeepGeneratorWidget) => {
                    dbgln!(audio, "Beep: {:?}", address);
                    self.beep_addr = Some(address);
                }
                Some(GetParameterAudioWidgetCapReg::TYPE::Value::PinComplex) => {
                    let cfg = node.config_default();
                    if cfg.is_input() {
                        dbgln!(audio, "Input Pin: {:?}", address);
                        self.input_pins.push(address);
                    } else if cfg.is_output() {
                        dbgln!(audio, "Output Pin: {:?}", address);
                        self.output_pins.push(address);
                    }
                }
                _ => {}
            }

            self.nodes.insert(address, node);
        }
    }

    fn enumerate_function_groups(&mut self, codec: u32, range: core::ops::Range<u64>) {
        for node in range {
            let address = Address::new(codec, node as u32);

            let node = node::Node::new(address, false, &mut self.cmd);

            let start_node = node.start_node();
            let last_node = start_node + node.node_count() as u32;

            self.enumerate_nodes(codec, start_node..last_node);
        }
    }

    fn enumerate(&mut self) {
        let codecs = self.reg.statest.sdiwake();

        for codec in 0u32..16 {
            if !codecs.get_bit(codec as usize) {
                continue;
            }
            let node = 0;
            let val = self
                .cmd
                .invoke::<GetParameterNodeCount>(Address::new(codec, node));

            let vid = val.starting_node();
            let did = val.total_count();

            dbgln!(audio, "function group range: {}..{}", vid, vid + did);

            self.enumerate_function_groups(codec, vid..vid + did);

            dbgln!(
                audio,
                "codec {} node {} start {} count {}",
                codec,
                node,
                vid,
                did
            );
        }
    }

    fn find_output_pin(&mut self) -> Option<Address> {
        if self.output_pins.len() == 0 {
            None
        } else if self.output_pins.len() == 1 {
            Some(self.output_pins[0])
        } else {
            for out in &self.output_pins {
                let node = self.nodes.get(out).unwrap();
                let cd = node.config_default();
                dbgln!(audio, "output pin: {:?}", cd.default_device());
            }
            use ConfigurationDefaultReg::DEFAULT_DEVICE;
            let supported_devs = &[DEFAULT_DEVICE::Value::HPOut, DEFAULT_DEVICE::Value::Speaker];

            for out in &self.output_pins {
                let node = self.nodes.get(out).unwrap();
                let cd = node.config_default();

                if cd.sequence() == 0 && supported_devs.contains(&cd.default_device().unwrap()) {
                    let pin_caps = self.cmd.invoke::<GetParameterPinCap>(*out);

                    if pin_caps.is_presence_detect_capable() {
                        let pin_sense = self.cmd.invoke::<GetPinSense>(*out);

                        if !pin_sense.is_presence_detected() {
                            continue;
                        }
                    }

                    return Some(*out);
                }
            }

            None
        }
    }

    fn find_path_to_dac(&self, addr: Address) -> Option<Vec<Address>> {
        let node = self.nodes.get(&addr)?;

        if node.capabilities().typ()? == GetParameterAudioWidgetCapReg::TYPE::Value::AudioOutput {
            Some(vec![addr])
        } else {
            let con_default = node.selected_connection();
            let mut path = self.find_path_to_dac(*node.connections().get(con_default as usize)?)?;
            path.insert(0, addr);
            Some(path)
        }
    }

    fn set_dpbase(&mut self, addr: PhysAddr) {
        self.reg.dplupbase.set(addr.0.get_bits(32..64) as u32);
        let mut low = self.reg.dpllowbase.get_local();
        low.set_base(addr.0 as u32);
        low.set_is_dma_pos_enabled(true);
        self.reg.dpllowbase.set(low.get());
    }

    fn init_posbuf(&mut self) {
        let posbuf = allocate_order(0).unwrap().address();

        map_to_flags(posbuf.to_virt(), posbuf, PageFlags::NO_CACHE);

        self.posbuf = posbuf;
        self.set_dpbase(posbuf);
    }

    pub fn init_stream_regs(&mut self) {
        let mut dpl_offset = self.posbuf.to_virt();
        let mut reg_offset = VirtAddr(self.reg as *const _ as usize) + 0x80;

        let mut init_streams = |count: usize| {
            for addr in (reg_offset..(reg_offset + 0x20 * count)).step_by(0x20) {
                self.streams.push(stream::Stream::new(addr, dpl_offset));

                dpl_offset += 8;
            }

            reg_offset += 0x20 * count;
        };

        init_streams(self.reg.gcap.iss() as usize);
        init_streams(self.reg.gcap.oss() as usize);
        init_streams(self.reg.gcap.bss() as usize);
    }

    fn configure(&mut self) {
        self.init_posbuf();
        self.init_stream_regs();

        let outpin = self.find_output_pin().expect("No output pins?");

        let path = self
            .find_path_to_dac(outpin)
            .expect("Failed to find path to dac");

        dbgln!(audio, "output path: {:?}", path);

        let pin = *path.first().unwrap();
        let dac = *path.last().unwrap();

        dbgln!(audio, "Pin: {:?}, Dac: {:?}", pin, dac);

        for &addr in &path {
            let mut reg = <SetPowerState as NodeCommand>::Data::new();

            // Fully on
            reg.set_ps_set(verb::PowerStateReg::PS_SET::Value::D0);
            self.cmd.invoke_data::<SetPowerState>(addr, reg);
        }

        dbgln!(audio, "Power State On!");

        // Pin enable
        let mut reg = <SetPinWidgetControl as NodeCommand>::Data::new();
        reg.set_is_out_enabled(true);
        reg.set_is_hphn_enabled(true);
        self.cmd.invoke_data::<SetPinWidgetControl>(pin, reg);

        dbgln!(audio, "Pin Enabled!");

        // EAPD enable
        let mut reg = <SetEAPDBTLEnable as NodeCommand>::Data::new();
        reg.set_is_eapd(true);
        self.cmd.invoke_data::<SetEAPDBTLEnable>(pin, reg);

        dbgln!(audio, "EAPD Enabled!");

        // Setup stream and channel (stream 0 is reserved by convention)
        let mut reg = <SetChannelStreamID as NodeCommand>::Data::new();
        reg.set_stream(1);
        reg.set_channel(0);
        self.cmd.invoke_data::<SetChannelStreamID>(dac, reg);

        let stream = self
            .out_stream(0)
            .expect("Failed to get output stream descriptor");
        stream.set_stream_number(1);
        stream.set_pcm_format(&stream::SR_44_1, reg::StreamFormat::BITS::Value::BITS16, 2);

        let mut reg = <SetConverterFormat as NodeCommand>::Data::new();
        reg.set_base(verb::StreamFormatReg::BASE::Value::KHZ44);
        reg.set_div(0);
        reg.set_mult(verb::StreamFormatReg::MULT::Value::NONE);
        reg.set_bits(verb::StreamFormatReg::BITS::Value::BITS16);
        reg.set_chan(1); // 2 - 1
        self.cmd.invoke_data::<SetConverterFormat>(dac, reg);

        let output = stream::OutputStream::new(stream).expect("Failed to create output stream");

        self.output_streams.push(output);

        let output = self.output_streams.last().unwrap();

        for &addr in &path {
            let node = self.nodes.get(&addr).unwrap();

            let caps = node.capabilities();

            if caps.is_in_amp_present() {
                let in_caps = self.cmd.invoke::<GetParameterInputAmplifierCap>(addr);
                let in_gain = in_caps.offset();
                let mut reg = <SetAmplifierGainMute as NodeCommand>::Data::new();

                reg.set_is_set_input_amp(true);
                reg.set_is_set_output_amp(false);
                reg.set_is_set_left_amp(true);
                reg.set_is_set_right_amp(true);
                reg.set_index(0);
                reg.set_is_mute(false);
                reg.set_gain(in_gain);

                self.cmd.invoke_data::<SetAmplifierGainMute>(addr, reg);
                dbgln!(audio, "Setting IN amp gain to {}", in_gain);
            }

            if caps.is_out_amp_present() {
                let out_caps = self.cmd.invoke::<GetParameterOutputAmplifierCap>(addr);
                let out_gain = out_caps.offset();
                let mut reg = <SetAmplifierGainMute as NodeCommand>::Data::new();

                reg.set_is_set_input_amp(false);
                reg.set_is_set_output_amp(true);
                reg.set_is_set_left_amp(true);
                reg.set_is_set_right_amp(true);
                reg.set_index(0);
                reg.set_is_mute(false);
                reg.set_gain(out_gain);

                self.cmd.invoke_data::<SetAmplifierGainMute>(addr, reg);

                dbgln!(audio, "Setting OUT amp gain to {}", out_gain);
            }
        }

        output.run();

        dbgln!(
            audio,
            "out stream reg address: {}",
            output.stream().reg_address()
        );

        dbgln!(audio, "Stream And Channel Set And Running!");
    }

    fn start_controller(&mut self) {
        let regs = &self.reg;

        regs.statest.set_sdiwake(0xffff);

        regs.gctl.set_is_crst(false);
        while regs.gctl.is_crst() {}

        regs.gctl.set_is_crst(true);
        while !regs.gctl.is_crst() {}

        dbgln!(audio, "controller initialised");

        while regs.statest.sdiwake() == 0 {}

        dbgln!(audio, "statest: {}", regs.statest.sdiwake());
    }

    fn start(&mut self, pci: &PciHeader) -> bool {
        self.start_controller();

        self.cmd.setup();

        let oss = self.reg.gcap.oss();
        let iss = self.reg.gcap.iss();
        let bss = self.reg.gcap.bss();

        dbgln!(audio, "oss: {}, iss: {}, bss: {}", oss, iss, bss);

        self.setup_interrupts(pci);

        self.enumerate();

        dbgln!(audio, "Enumerate done, configure start!");

        self.configure();

        true
    }
}

struct IntelHda {
    self_ref: Weak<IntelHda>,
    id: DevId,
    data: Spin<IntelHdaData>,
    wq: WaitQueue,
}

impl IntelHda {
    pub fn new(base_addr: VirtAddr) -> Arc<IntelHda> {
        Arc::new_cyclic(|me| IntelHda {
            self_ref: me.clone(),
            id: crate::kernel::device::alloc_id(),
            data: Spin::new(IntelHdaData::new(base_addr)),
            wq: WaitQueue::new(),
        })
    }

    pub fn start(&self, pci_data: &PciHeader) -> bool {
        if if let PciHeader::Type0(_dhdr) = pci_data {
            pci_data.hdr().enable_bus_mastering();

            true
        } else {
            false
        } {
            let mut data = self.data.lock_irq();

            dbgln!(audio, "Starting intel hd audio");

            let res = data.start(pci_data);

            println!("[ OK ] HD Audio started!");

            res
        } else {
            false
        }
    }

    fn handle_interrupt(&self) -> bool {
        self.data.lock_irq().handle_interrupt(&self.wq)
    }
}

impl Device for IntelHda {
    fn id(&self) -> DevId {
        self.id
    }

    fn name(&self) -> String {
        "hda".into()
    }

    fn inode(&self) -> Arc<dyn INode> {
        self.self_ref.upgrade().unwrap()
    }
}

impl INode for IntelHda {
    fn poll(
        &self,
        poll_table: Option<&mut PollTable>,
        _flags: PollEventFlags,
    ) -> crate::kernel::fs::vfs::Result<PollEventFlags> {
        // Wait for an output buffer pos change
        // bit of a hack, let it sleep and report ready after we get notify
        // TODO: Think of a better way
        if let Some(pt) = poll_table {
            pt.listen(&self.wq);

            Ok(PollEventFlags::empty())
        } else {
            Ok(PollEventFlags::WRITE)
        }
    }

    fn as_mappable(&self) -> Option<Arc<dyn MappedAccess>> {
        if let Some(me) = self.self_ref.upgrade() {
            return Some(me);
        }

        None
    }
}

impl MappedAccess for IntelHda {
    fn get_mmap_page(&self, mut offset: usize, _size_check: bool) -> Option<MMapPageStruct> {
        let data = self.data.lock_irq();

        offset = offset.align_down(PAGE_SIZE);

        if offset == 0 {
            Some(MMapPageStruct(MMapPage::Direct(PageDirectItemStruct::new(
                data.posbuf,
                offset,
                PageFlags::NO_CACHE,
            ))))
        } else {
            offset -= PAGE_SIZE;
            let stream = data.output_streams.first()?;

            let buffer = stream.buffer();

            if offset >= buffer.buffer_size() {
                None
            } else {
                Some(MMapPageStruct(MMapPage::Direct(PageDirectItemStruct::new(
                    (buffer.buffer_address() + offset).to_phys(),
                    offset,
                    PageFlags::NO_CACHE,
                ))))
            }
        }
    }
}

struct IntelHdaPciDevice {}

impl PciDeviceHandle for IntelHdaPciDevice {
    fn handles(&self, pci_vendor_id: u64, pci_dev_id: u64) -> bool {
        match (pci_vendor_id, pci_dev_id) {
            (0x8086, 0x2668) => true,
            _ => false,
        }
    }

    fn start(&self, pci_data: &PciHeader) -> bool {
        dbgln!(audio, "Starting Intel HD Audio!");
        let ba = pci_data.try_hdr0().unwrap().base_address0();
        dbgln!(audio, "base addr 0 {}", ba.address());
        dbgln!(audio, "is64: {}", ba.is64());
        dbgln!(audio, "isIO: {}", ba.is_io());
        dbgln!(audio, "is_prefetch: {}", ba.is_prefetchable());

        let base_addr = ba.address_map_virt();

        DEVICE.call_once(|| IntelHda::new(base_addr));

        if device().start(pci_data) {
            register_device(device().clone()).expect("Failed to register HDA device");

            true
        } else {
            false
        }
    }
}

static DEVICE: Once<Arc<IntelHda>> = Once::new();

fn device() -> &'static Arc<IntelHda> {
    unsafe { DEVICE.get_unchecked() }
}

fn init() {
    register_pci_device(Arc::new(IntelHdaPciDevice {}));
}

module_init!(init);
