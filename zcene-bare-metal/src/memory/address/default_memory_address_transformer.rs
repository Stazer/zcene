#[cfg(target_arch = "x86_64")]
use crate::memory::address::TruncatedBitsMemoryAddressTransformer;
#[cfg(not(target_arch = "x86_64"))]
use crate::memory::address::IdentityMemoryAddressTransformer;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(target_arch = "x86_64")]
pub type DefaultMemoryAddressTransformer = TruncatedBitsMemoryAddressTransformer<52>;

#[cfg(not(target_arch = "x86_64"))]
pub type DefaultMemoryAddressTransformer = IdentityMemoryAddressTransformer;
