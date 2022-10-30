use std::marker::PhantomData;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}

pub struct Dict<V, const BLOCKSIZE: usize> {
    keys: Vec<[Vec<u8>; BLOCKSIZE]>,
    values: Vec<V>,
    current_block: Vec<V>,
}
