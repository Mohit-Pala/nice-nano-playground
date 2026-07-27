// use a lifetime param here, cus radioData's payload borrows data from rx buf
// rx buf needs to be static lifetime because of DMA
pub struct ScRadioData<'a> {
    pub crc_ok: bool,
    pub s1_pid: u8,
    pub payload: &'a [u8],
}

impl<'a> ScRadioData<'a> {
    pub fn from_buf(buf: &'a [u8], crc_ok: bool) -> Self {
        let buf_len = buf[0] as usize;
        // min of payload len vs 100 - 2
        let payload_len = buf_len.min(buf.len().saturating_sub(2));
        Self {
            crc_ok,
            s1_pid: buf[1],
            payload: &buf[2..2 + payload_len] // array slice from idx 2 to 2 + 17? 
        }
    }
}