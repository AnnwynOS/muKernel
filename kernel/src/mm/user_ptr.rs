pub const USER_SPACE_END: u64 = 0x0000_8000_0000_0000;

#[derive(Debug, Clone, Copy)]
pub enum UserPtrError {
    Null,
    KernelSpace,
    Overflow,
    CrossesBoundary,
    TooLarge,
}

pub const MAX_SYSCALL_TRANSFER: usize = 64 * 1024;

pub fn validate_user_read(ptr: u64, len: usize) -> Result<(), UserPtrError> {
    validate_range(ptr, len)
}

pub fn validate_user_write(ptr: u64, len: usize) -> Result<(), UserPtrError> {
    validate_range(ptr, len)
}

pub fn validate_user_str(ptr: u64, len: usize) -> Result<&'static [u8], UserPtrError> {
    validate_range(ptr, len)?;
    Ok(unsafe { core::slice::from_raw_parts(ptr as *const u8, len) })
}

pub fn read_user<T: Copy>(ptr: u64) -> Result<T, UserPtrError> {
    validate_range(ptr, core::mem::size_of::<T>())?;
    Ok(unsafe { core::ptr::read_unaligned(ptr as *const T) })
}

pub fn write_user<T: Copy>(ptr: u64, val: T) -> Result<(), UserPtrError> {
    validate_range(ptr, core::mem::size_of::<T>())?;
    unsafe { core::ptr::write_unaligned(ptr as *mut T, val) };
    Ok(())
}

fn validate_range(ptr: u64, len: usize) -> Result<(), UserPtrError> {
    if ptr == 0 {
        return Err(UserPtrError::Null);
    }
    if len > MAX_SYSCALL_TRANSFER {
        return Err(UserPtrError::TooLarge);
    }
    let end = ptr.checked_add(len as u64)
        .ok_or(UserPtrError::Overflow)?;

    if ptr >= USER_SPACE_END {
        return Err(UserPtrError::KernelSpace);
    }
    if end > USER_SPACE_END {
        return Err(UserPtrError::CrossesBoundary);
    }

    Ok(())
}