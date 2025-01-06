pub unsafe fn linker_value(value: &usize) -> usize {
    value as *const _ as usize
}
