#![allow(dead_code)]

use crate::drivers::audio::hda::cmd::Command;
use crate::drivers::audio::hda::reg::verb::{
    ConfigurationDefaultReg, ConnectionListEntryInt, ConnectionListEntryKind, FunctionGroupType,
    GetConfigurationDefault, GetConnectionListEntry, GetConnectionSelectionControl,
    GetParameterAudioWidgetCap, GetParameterAudioWidgetCapReg, GetParameterConnectionListLength,
    GetParameterConnectionListLengthReg, GetParameterFunctionGroupType, GetParameterNodeCount,
};
use crate::drivers::audio::hda::reg::WrapLocal;
use crate::drivers::audio::hda::Address;
use alloc::boxed::Box;
use alloc::vec::Vec;

pub struct Node {
    address: Address,

    start_node: u32,
    node_count: usize,
    function_group_type: FunctionGroupType,
    capabilities: WrapLocal<u64, GetParameterAudioWidgetCapReg::Register>,
    connection_list_len: WrapLocal<u64, GetParameterConnectionListLengthReg::Register>,
    connections: Vec<Address>,
    selected_connection: u8,
    is_widget: bool,
    config_default: WrapLocal<u64, ConfigurationDefaultReg::Register>,
}

impl Node {
    pub fn new(addr: Address, is_widget: bool, command: &mut Command) -> Node {
        let node_count_reg = command.invoke::<GetParameterNodeCount>(addr);
        let function_group = command
            .invoke::<GetParameterFunctionGroupType>(addr)
            .function_group_type();
        let capabilities = command.invoke::<GetParameterAudioWidgetCap>(addr);
        let config_default = command.invoke::<GetConfigurationDefault>(addr);
        let selected_connection = command.invoke::<GetConnectionSelectionControl>(addr) as u8;

        dbgln!(audio, "Reading node {:?} is_widget {}", addr, is_widget);
        dbgln!(audio, "Function group: {:?}", function_group);
        dbgln!(audio, "Type: {:?}", capabilities.typ());
        dbgln!(audio, "Selected Connection: {:?}", selected_connection);

        let mut node = Node {
            address: addr,
            start_node: node_count_reg.starting_node() as u32,
            node_count: node_count_reg.total_count() as usize,
            function_group_type: function_group,
            connection_list_len: WrapLocal::from(0),
            connections: Vec::new(),
            selected_connection,
            capabilities,
            is_widget,
            config_default,
        };

        node.connection_list_len =
            command.invoke::<GetParameterConnectionListLength>(node.address());

        if node.connection_list_len.is_long_form() {
            node.init_connection_list::<u16>(command);
        } else {
            node.init_connection_list::<u8>(command);
        }

        for c in &node.connections {
            dbgln!(audio, "Connection: {:?}", c);
        }

        node
    }

    fn init_connection_list<I: ConnectionListEntryInt>(&mut self, command: &mut Command) {
        let count = self.connection_list_len.length();

        dbgln!(
            audio,
            "Connection list length: {} long_form: {}",
            count,
            I::IS_LONG
        );

        let mut current = 0;

        fn process_entry(this: &mut Node, entry: Box<dyn ConnectionListEntryKind>) -> bool {
            if !entry.is_valid() {
                return false;
            }

            if !entry.is_range_of_nids() {
                this.connections
                    .push(Address::new(this.address.codec(), entry.nids() as u32));
            } else {
                let last = this.connections.pop().expect("Invalid Range List Entry");

                for i in last.node()..=entry.nids() as u32 {
                    this.connections.push(Address::new(this.address.codec(), i));
                }
            }

            true
        }

        let count_per_index = I::COUNT_PER_ENTRY;

        while current < count {
            let res = command.invoke_data::<GetConnectionListEntry>(self.address, current as u32);

            let mut idx = 0;

            while idx < count_per_index && process_entry(self, res.entry::<I>(idx)) {
                idx += 1;
            }

            if idx < count_per_index {
                break;
            }

            current += count_per_index as u64;
        }
    }

    pub fn address(&self) -> Address {
        self.address
    }

    pub fn start_node(&self) -> u32 {
        self.start_node
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn function_group_type(&self) -> FunctionGroupType {
        self.function_group_type
    }

    pub fn capabilities(&self) -> WrapLocal<u64, GetParameterAudioWidgetCapReg::Register> {
        self.capabilities
    }

    pub fn connection_list_len(
        &self,
    ) -> WrapLocal<u64, GetParameterConnectionListLengthReg::Register> {
        self.connection_list_len
    }

    pub fn connections(&self) -> &Vec<Address> {
        &self.connections
    }

    pub fn selected_connection(&self) -> u8 {
        self.selected_connection
    }

    pub fn is_widget(&self) -> bool {
        self.is_widget
    }

    pub fn config_default(&self) -> WrapLocal<u64, ConfigurationDefaultReg::Register> {
        self.config_default
    }
}
