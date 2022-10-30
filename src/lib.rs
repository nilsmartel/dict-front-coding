use std::cmp::Ordering;

use varint_compression::decompress;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prefix_len_1() {
        let values = [b"aaa".to_vec(), b"aaa".to_vec(), b"aaa".to_vec()];

        let expected = 3;
        let got = common_prefix_len(&values);

        assert_eq!(expected, got)
    }

    #[test]
    fn prefix_len_2() {
        let values = [b"aaaaa".to_vec(), b"aaabc".to_vec(), b"aaaf".to_vec()];

        let expected = 3;
        let got = common_prefix_len(&values);

        assert_eq!(expected, got)
    }

    #[test]
    fn prefix_len_3() {
        let values = [b"aaaaa".to_vec(), b"b".to_vec(), b"aaaf".to_vec()];

        let expected = 0;
        let got = common_prefix_len(&values);

        assert_eq!(expected, got)
    }

    #[test]
    fn block_compression() {
        let input = vec![
            b"aal".to_vec(),
            b"aachen".to_vec(),
            b"aachja".to_vec(),
            b"aadfg".to_vec(),
        ];

        let got = Block::<4>::new(&input).to_vec();

        assert_eq!(input, got);
    }

    #[test]
    fn retrieval() {
        let input = "a, aaa, aa, a, a, a ,a, aachen, aanderthen, aalamabama, aaalligator, behemeoth, bet, bed, ber, bar"
            .split(',')
            .map(|s| s.trim().as_bytes().to_vec());
        
        let mut input: Vec<_> = input.collect();
        input.sort();

        let mut dict = Dict::<(), 4>::new();

        for elem in input {
            dict.push(elem, ());
        }

        let index = dict.index_of(b"bar");
        assert!(index.is_some(), "element can be found in dictionary");
    }
}

pub struct Dict<V, const BLOCKSIZE: usize> {
    keys: Vec<Block<BLOCKSIZE>>,
    values: Vec<V>,
    current_block: Vec<(Vec<u8>, V)>,
}

impl<V, const B: usize> Dict<V, B>
where
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: Vec::new(),
            current_block: Vec::new(),
        }
    }

    pub fn values(&self) -> &[V] {
        &self.values
    }

    pub fn index_of(&self, key: &[u8]) -> Option<usize> {
        fn binary_search<const B: usize>(data: &[Block<B>], elem: &[u8]) -> Option<usize> {
            if data.is_empty() {
                return None;
            }

            let index = data.len() / 2;

            Some(match data[index].cmp(elem) {
                Ordering::Equal => index,
                Ordering::Less => index + 1 + binary_search(&data[(index + 1)..], elem)?,
                Ordering::Greater => binary_search(&data[..index], elem)?,
            })
        }

        let block_id = binary_search(&self.keys, key)?;

        for (i, v) in self.keys[block_id].to_vec().into_iter().enumerate() {
            if v == key {
                return Some(block_id * B + i);
            }
        }

        None
    }

    pub fn push(&mut self, key: Vec<u8>, value: V) {
        // actually it is vital to assert that our input data is sorted at this point.

        self.current_block.push((key, value));

        if self.current_block.len() == B {
            let values = self
                .current_block
                .iter()
                .map(|elem| &elem.0)
                .cloned()
                .collect::<Vec<_>>();

            self.values
                .extend(self.current_block.iter().map(|elem| elem.1.clone()));

            let block = Block::<B>::new(&values);
            self.keys.push(block);
            self.current_block.clear();
        }
    }
}

struct Block<const BLOCKSIZE: usize> {
    /*
        Scheme: prefix: <varint>data
        for b in Blocksize:
            <varint>data
    */
    data: Vec<u8>,
}

fn common_prefix_len(values: &[Vec<u8>]) -> usize {
    let mut prefix = &values[0] as &[u8];

    for v in values.iter().skip(1) {
        for i in 0..prefix.len() {
            if v.len() <= i || prefix[i] != v[i] {
                prefix = &prefix[..i];
                break;
            }
        }
    }

    prefix.len()
}

impl<const B: usize> Block<B> {
    fn cmp(&self, other: &[u8]) -> Ordering {
        let values = self.to_vec();

        match (&values[0] as &[u8]).cmp(other) {
            Ordering::Greater => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
            Ordering::Less => match (&values[B - 1] as &[u8]).cmp(other) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal => Ordering::Equal,
                Ordering::Greater => Ordering::Equal,
            },
        }
    }

    fn new(values: &[Vec<u8>]) -> Self {
        use varint_compression::*;

        assert_eq!(
            values.len(),
            B,
            "expect size of values to be equal to block size"
        );

        let prefixlen = common_prefix_len(values);

        let mut data = Vec::new();

        data.extend(compress(prefixlen as u64));
        data.extend(&values[0][..prefixlen]);

        for v in values {
            let v = &v[prefixlen..];
            data.extend(compress(v.len() as u64));
            data.extend(v);
        }

        Block { data }
    }

    fn to_vec(&self) -> Vec<Vec<u8>> {
        let input = &self.data;

        let (n, rest) = decompress(input).unwrap();
        let n = n as usize;

        let prefix = &rest[..n];
        let mut input = &rest[n..];

        let mut v = Vec::with_capacity(B);

        for _ in 0..B {
            let (n, rest) = decompress(input).unwrap();
            let n = n as usize;
            let suffix = &rest[..n];

            let mut value = Vec::with_capacity(prefix.len() + suffix.len());
            value.extend(prefix);
            value.extend(suffix);

            v.push(value);
            input = &rest[n..];
        }

        v
    }
}
