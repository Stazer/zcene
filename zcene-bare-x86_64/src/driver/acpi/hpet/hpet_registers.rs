use zcene_bare::common::volatile::{ReadVolatile, ReadWriteVolatile};
use ztd::Method;

///////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Method)]
#[repr(C)]
pub struct HpetRegisters {
    #[Method(accessor)]
    general_capabilities_and_id: ReadVolatile<u64>,
    _reserved0: u64,

    #[Method(accessor, mutator)]
    general_configuration: ReadWriteVolatile<u64>,
    _reserved1: u64,

    #[Method(accessor, mutator)]
    general_interrupt_status: ReadWriteVolatile<u64>,
    _reserved2: [u64; 25],

    #[Method(accessor, mutator)]
    main_counter_value: ReadWriteVolatile<u64>,
    _reserved3: u64,
}
