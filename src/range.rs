use failure::bail;
use reqwest::header::HeaderValue;

pub type Error<T> = Result<T, failure::Error>;

pub struct PartialRangeIter {
    start: u64,
    end: u64,
    buffer_size: usize,
}

impl PartialRangeIter {
    pub fn new(start: u64, end: u64, buffer_size: usize) -> Error<Self> {
        if buffer_size == 0 {
            bail!("invalid buffer_size, give a value greater than zero.");
        }

        Ok(PartialRangeIter {
            start,
            end,
            buffer_size,
        })
    }
}

impl Iterator for PartialRangeIter {
    type Item = HeaderValue;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start > self.end {
            None
        } else {
            let prev_start = self.start;
            self.start += std::cmp::min(self.buffer_size as u64, self.end - self.start + 1);
            // NOTE(unwrap): `HeaderValue::from_str` will fail only if the value is not made
            // of visible ASCII characters. Since the format string is static and the two
            // values are integers, that can't happen.
            Some(
                HeaderValue::from_str(&format!("bytes={}-{}", prev_start, self.start - 1)).unwrap(),
            )
        }
    }
}
