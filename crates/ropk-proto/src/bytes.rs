// helper methods from triton.h

pub fn array<const N: usize>(b: &[u8], off: usize) -> Option<[u8; N]> {    
    let end = off.checked_add(N)?;
    if end > b.len() {
        return None;
    }
    let mut result = [0u8; N];
    result.copy_from_slice(&b[off..end]);
    Some(result)
}

pub fn u16_le(b: &[u8], off: usize) -> Option<u16> {
    array(b, off).map(u16::from_le_bytes)
}

pub fn i16_le(b: &[u8], off: usize) -> Option<i16> {
    array(b, off).map(i16::from_le_bytes)
}

pub fn u32_le(b: &[u8], off: usize) -> Option<u32> {
    array(b, off).map(u32::from_le_bytes)
}