#[derive(Debug, Copy, Clone)]
pub struct RelaisMsg {}

impl RelaisMsg {
    pub fn parse(data: &[u8]) -> Option<(usize, bool, Option<u64>)> {
        if data.len() < 2 {
            return None;
        }

        let number = data[0] as usize;
        let state = data[1] != 0;

        if data.len() >= 5 {
            let time: u64 = (data[2] as u64) | ((data[3] as u64) << 8) | ((data[4] as u64) << 16);
            Some((number, state, Some(time)))
        } else {
            Some((number, state, None))
        }
    }
}
