#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;

fn main(args: eonos_app::Args<'_>) -> i32 {
    let mut args = args.skip(1);
    let Some(path) = args.next() else {
        eonos_app::println!("usage: sha256 <file>");
        return 2;
    };
    if args.next().is_some() {
        eonos_app::println!("usage: sha256 <file>");
        return 2;
    }
    let path = match path.to_str() {
        Ok(path) => path,
        Err(_) => {
            eonos_app::println!("sha256: file path is not valid UTF-8");
            return 2;
        }
    };
    let contents = match eonos_app::system::read_file(path) {
        Ok(contents) => contents,
        Err(error) => {
            eonos_app::println!("sha256: cannot read {}: {:?}", path, error);
            return 1;
        }
    };

    let digest = sha256(&contents);
    let digits = b"0123456789abcdef";
    let mut output = [0u8; 64];
    for (index, byte) in digest.iter().enumerate() {
        output[index * 2] = digits[(byte >> 4) as usize];
        output[index * 2 + 1] = digits[(byte & 0x0f) as usize];
    }
    let hex = core::str::from_utf8(&output).unwrap_or("<format error>");
    eonos_app::println!("{}  {}", hex, path);
    0
}

fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state = [
        0x6a09e667u32, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let block_count = (data.len() + 9).div_ceil(64);

    for block_index in 0..block_count {
        let block_start = block_index * 64;
        let mut block = [0u8; 64];
        if block_start < data.len() {
            let amount = (data.len() - block_start).min(64);
            block[..amount].copy_from_slice(&data[block_start..block_start + amount]);
        }
        if data.len() >= block_start && data.len() < block_start + 64 {
            block[data.len() - block_start] = 0x80;
        }
        if block_index + 1 == block_count {
            let bit_length = (data.len() as u64).wrapping_mul(8).to_be_bytes();
            block[56..64].copy_from_slice(&bit_length);
        }

        let mut words = [0u32; 64];
        for index in 0..16 {
            let offset = index * 4;
            words[index] = u32::from_be_bytes([
                block[offset], block[offset + 1], block[offset + 2], block[offset + 3],
            ]);
        }
        for index in 16..64 {
            let x = words[index - 15];
            let y = words[index - 2];
            let small0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let small1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            words[index] = words[index - 16]
                .wrapping_add(small0)
                .wrapping_add(words[index - 7])
                .wrapping_add(small1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
        for index in 0..64 {
            let big1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(big1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let big0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = big0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        for (value, addition) in state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *value = value.wrapping_add(addition);
        }
    }

    let mut output = [0u8; 32];
    for (index, value) in state.iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
    }
    output
}

eonos_app::app!(SHELL, main);
