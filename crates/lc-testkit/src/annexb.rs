use std::ops::Range;

/// Tipos de NAL presentes em `data`, na ordem.
pub fn nal_types(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 3 < data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            out.push(data[i + 3] & 0x1F);
            i += 3;
        } else {
            i += 1;
        }
    }
    out
}

/// Intervalos de cada access unit (cada AU começa em um AUD, NAL tipo 9).
pub fn split_access_units(data: &[u8]) -> Vec<Range<usize>> {
    let mut starts = Vec::new();
    let mut i = 0;
    while i + 3 < data.len() {
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 1 {
            if data[i + 3] & 0x1F == 9 {
                let s = if i > 0 && data[i - 1] == 0 { i - 1 } else { i };
                starts.push(s);
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    let mut ranges = Vec::new();
    for (k, &s) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(data.len());
        ranges.push(s..end);
    }
    ranges
}

/// true se o AU contém um slice IDR (tipo 5).
pub fn is_idr(au: &[u8]) -> bool {
    nal_types(au).contains(&5)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3 AUs sintéticos: AU0 = AUD+SPS+PPS+IDR (4-byte start codes),
    /// AU1 = AUD+slice, AU2 = AUD+slice com start code de 3 bytes.
    fn sample() -> Vec<u8> {
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(&[0, 0, 0, 1, 0x09, 0x10]);
        b.extend_from_slice(&[0, 0, 0, 1, 0x67, 0x11]);
        b.extend_from_slice(&[0, 0, 0, 1, 0x68, 0x12]);
        b.extend_from_slice(&[0, 0, 0, 1, 0x65, 0xAA, 0xBB]);
        b.extend_from_slice(&[0, 0, 0, 1, 0x09, 0x30]);
        b.extend_from_slice(&[0, 0, 0, 1, 0x41, 0xCC]);
        b.extend_from_slice(&[0, 0, 0, 1, 0x09, 0x30]);
        b.extend_from_slice(&[0, 0, 0, 1, 0x41, 0xDD]);
        b
    }

    #[test]
    fn splits_three_aus() {
        let data = sample();
        let ranges = split_access_units(&data);
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[0].start, 0);
        assert_eq!(ranges[2].end, data.len());
    }

    #[test]
    fn only_first_au_is_idr() {
        let data = sample();
        let ranges = split_access_units(&data);
        assert!(is_idr(&data[ranges[0].clone()]));
        assert!(!is_idr(&data[ranges[1].clone()]));
        assert!(!is_idr(&data[ranges[2].clone()]));
    }

    #[test]
    fn first_au_nal_types_are_aud_sps_pps_idr() {
        let data = sample();
        let ranges = split_access_units(&data);
        assert_eq!(nal_types(&data[ranges[0].clone()]), vec![9, 7, 8, 5]);
        assert_eq!(nal_types(&data[ranges[1].clone()]), vec![9, 1]);
        assert_eq!(nal_types(&data[ranges[2].clone()]), vec![9, 1]);
    }

    #[test]
    fn ranges_are_contiguous_and_cover_buffer() {
        let data = sample();
        let ranges = split_access_units(&data);
        assert_eq!(ranges[0].end, ranges[1].start);
        assert_eq!(ranges[1].end, ranges[2].start);
    }
}
