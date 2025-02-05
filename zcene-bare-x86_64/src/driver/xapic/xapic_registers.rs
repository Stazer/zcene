use zcene_bare::common::volatile::{ReadVolatile, ReadWriteVolatile, WriteVolatile};
use ztd::Method;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Method)]
#[repr(C)]
pub struct XApicRegisters {
    _padding0: [u32; 8],

    #[Method(accessor)]
    lapic_id: ReadVolatile<u32>,
    _padding1: [u32; 3],

    #[Method(accessor)]
    lapic_version: ReadVolatile<u32>,
    _padding2: [u32; 3],

    _padding3: [u32; 16],

    #[Method(accessor, mutator)]
    task_priority: ReadWriteVolatile<u32>,
    _padding4: [u32; 3],

    #[Method(accessor)]
    arbitration_priority: ReadVolatile<u32>,
    _padding5: [u32; 3],

    #[Method(accessor)]
    processor_priority: ReadVolatile<u32>,
    _padding6: [u32; 3],

    #[Method(mutator)]
    eoi: WriteVolatile<u32>,
    _padding7: [u32; 3],

    #[Method(accessor)]
    remote_read: ReadVolatile<u32>,
    _padding8: [u32; 3],

    #[Method(accessor, mutator)]
    logical_destination: ReadWriteVolatile<u32>,
    _padding9: [u32; 3],

    #[Method(accessor, mutator)]
    destination_format: ReadWriteVolatile<u32>,
    _padding10: [u32; 3],

    #[Method(accessor, mutator)]
    spurious_interrupt_vector: ReadWriteVolatile<u32>,
    _padding11: [u32; 3],

    #[Method(accessor)]
    in_service_register0: ReadVolatile<u32>,
    _padding12: [u32; 3],

    #[Method(accessor)]
    in_service_register1: ReadVolatile<u32>,
    _padding13: [u32; 3],

    #[Method(accessor)]
    in_service_register2: ReadVolatile<u32>,
    _padding14: [u32; 3],

    #[Method(accessor)]
    in_service_register3: ReadVolatile<u32>,
    _padding15: [u32; 3],

    #[Method(accessor)]
    in_service_register4: ReadVolatile<u32>,
    _padding16: [u32; 3],

    #[Method(accessor)]
    in_service_register5: ReadVolatile<u32>,
    _padding17: [u32; 3],

    #[Method(accessor)]
    in_service_register6: ReadVolatile<u32>,
    _padding18: [u32; 3],

    #[Method(accessor)]
    in_service_register7: ReadVolatile<u32>,
    _padding19: [u32; 3],

    #[Method(accessor)]
    trigger_mode_register0: ReadVolatile<u32>,
    _padding20: [u32; 3],

    #[Method(accessor)]
    trigger_mode_register1: ReadVolatile<u32>,
    _padding21: [u32; 3],

    #[Method(accessor)]
    trigger_mode_register2: ReadVolatile<u32>,
    _padding22: [u32; 3],

    #[Method(accessor)]
    trigger_mode_register3: ReadVolatile<u32>,
    _padding23: [u32; 3],

    #[Method(accessor)]
    trigger_mode_register4: ReadVolatile<u32>,
    _padding24: [u32; 3],

    #[Method(accessor)]
    trigger_mode_register5: ReadVolatile<u32>,
    _padding25: [u32; 3],

    #[Method(accessor)]
    trigger_mode_register6: ReadVolatile<u32>,
    _padding26: [u32; 3],

    #[Method(accessor)]
    trigger_mode_register7: ReadVolatile<u32>,
    _padding27: [u32; 3],

    #[Method(accessor)]
    interrupt_request_register0: ReadVolatile<u32>,
    _padding28: [u32; 3],

    #[Method(accessor)]
    interrupt_request_register1: ReadVolatile<u32>,
    _padding29: [u32; 3],

    #[Method(accessor)]
    interrupt_request_register2: ReadVolatile<u32>,
    _padding30: [u32; 3],

    #[Method(accessor)]
    interrupt_request_register3: ReadVolatile<u32>,
    _padding31: [u32; 3],

    #[Method(accessor)]
    interrupt_request_register4: ReadVolatile<u32>,
    _padding32: [u32; 3],

    #[Method(accessor)]
    interrupt_request_register5: ReadVolatile<u32>,
    _padding33: [u32; 3],

    #[Method(accessor)]
    interrupt_request_register6: ReadVolatile<u32>,
    _padding34: [u32; 3],

    #[Method(accessor)]
    interrupt_request_register7: ReadVolatile<u32>,
    _padding35: [u32; 3],

    #[Method(accessor)]
    error_status: ReadVolatile<u32>,
    _padding36: [u32; 3],

    _padding37: [u32; 24],

    #[Method(accessor, mutator)]
    lvt_cmci: ReadWriteVolatile<u32>,
    _padding38: [u32; 3],

    #[Method(accessor, mutator)]
    interrupt_command_low: ReadWriteVolatile<u32>,
    _padding39: [u32; 3],

    #[Method(accessor, mutator)]
    interrupt_command_high: ReadWriteVolatile<u32>,
    _padding40: [u32; 3],

    #[Method(accessor, mutator)]
    lvt_timer: ReadWriteVolatile<u32>,
    _padding41: [u32; 3],

    #[Method(accessor, mutator)]
    lvt_thermal: ReadWriteVolatile<u32>,
    _padding42: [u32; 3],

    #[Method(accessor, mutator)]
    lvt_perf_monitor: ReadWriteVolatile<u32>,
    _padding43: [u32; 3],

    #[Method(accessor, mutator)]
    lvt_lint0: ReadWriteVolatile<u32>,
    _padding44: [u32; 3],

    #[Method(accessor, mutator)]
    lvt_lint1: ReadWriteVolatile<u32>,
    _padding45: [u32; 3],

    #[Method(accessor, mutator)]
    lvt_error: ReadWriteVolatile<u32>,
    _padding46: [u32; 3],

    #[Method(accessor, mutator)]
    timer_initial_count: ReadWriteVolatile<u32>,
    _padding47: [u32; 3],

    #[Method(accessor)]
    timer_current_count: ReadVolatile<u32>,
    _padding48: [u32; 3],

    _padding49: [u32; 16],

    #[Method(accessor, mutator)]
    timer_divide: ReadWriteVolatile<u32>,
    _padding50: [u32; 3],

    _padding51: [u32; 4],
}
