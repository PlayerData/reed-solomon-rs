use core::convert::TryInto;
use super::gf::poly_math::*;
use super::gf;
use heapless::Vec;

/// Reed-Solomon BCH encoder
#[derive(Debug)]
pub struct Encoder<const ECC_BYTE_COUNT_STORE: usize> {
    generator: [u8; ECC_BYTE_COUNT_STORE],
    lgenerator: [u8; ECC_BYTE_COUNT_STORE],
    scratch_space: Vec<u8, ECC_BYTE_COUNT_STORE>,
    bytes_processed: u8,
}

impl<const ECC_BYTE_COUNT_STORE: usize> Encoder<ECC_BYTE_COUNT_STORE> {
    const fn make_lgenerator(generator: &[u8; ECC_BYTE_COUNT_STORE]) -> [u8; ECC_BYTE_COUNT_STORE] {
        let mut lgen = [0u8; ECC_BYTE_COUNT_STORE];
        let mut i = 0;
        while i < generator.len() {
            lgen[i] = gf::LOG[generator[i] as usize];
            i += 1;
        }
        lgen
    }

    /// Constructs a new `Encoder` and calculates generator polynomial of given `ecc_len`.
    ///
    /// # Example
    /// ```rust
    /// use reed_solomon::Encoder;
    ///
    /// let encoder = Encoder::<9>::new(8);
    /// ```
    pub fn new(ecc_len: usize) -> Self {
        debug_assert!(ecc_len == ECC_BYTE_COUNT_STORE - 1, "ECC length must be ECC_BYTE_COUNT_STORE - 1");
        let generator: [u8; ECC_BYTE_COUNT_STORE] = generator_poly(ecc_len).try_into().unwrap();

        Self::new_with_precomputed_generator(&generator)
    }


    // generator_poly should be called to produce an array to be passed into this function
    // The array should be ecc_len + 1 bytes long
    pub const fn new_with_precomputed_generator(generator: &[u8; ECC_BYTE_COUNT_STORE]) -> Self {
        Self {
            lgenerator: Self::make_lgenerator(generator),
            generator: *generator,
            scratch_space: Vec::new(),
            bytes_processed: 0,
        }
    }

    /// Encodes passed `&[u8]` slice and returns `Buffer` with result and `ecc` offset.
    ///
    /// # Example
    /// ```rust
    /// use reed_solomon::Encoder;
    ///
    /// let data = "Hello World".as_bytes();
    /// let mut encoder = Encoder::<9>::new(8);
    ///
    /// let encoded = encoder.encode(&data);
    ///
    /// println!("ecc:   {:?}", encoded);
    /// ```
    pub fn encode(&mut self, data: &[u8]) -> Vec<u8, ECC_BYTE_COUNT_STORE> {
        debug_assert!(data.len() < 256 - self.generator.len(), "Data isnt a single chunk long or less");
        let mut ecc = Vec::<u8, ECC_BYTE_COUNT_STORE>::new();
        for byte in data.iter() {
            ecc = self.encode_single(*byte);
        }
        if let Ok(ecc) = self.finalize() {
            ecc
        } else {
            ecc
        }
    }

    pub fn encode_single(&mut self, data: u8) -> Vec<u8, ECC_BYTE_COUNT_STORE> {
        //First fill up scratch space
        if self.scratch_space.len() < self.generator.len() {
            unsafe { self.scratch_space.push(data).unwrap_unchecked() };
            self.bytes_processed += 1;
            return unsafe { Vec::from_slice(&[data]).unwrap_unchecked() };
        }

        self.run_encoding_round();

        self.scratch_space.rotate_left(1);
        self.scratch_space[self.generator.len() - 1] = data;

        self.bytes_processed += 1;
        if self.bytes_processed == (256 - self.generator.len()) as u8 {
            let mut ecc = unsafe { self.finalize().unwrap_unchecked() };
            unsafe { ecc.insert(0, data).unwrap_unchecked() };
            return ecc;
        }

        unsafe { Vec::from_slice(&[data]).unwrap_unchecked() }
    }

    // Errors if nothing in scratch space
    pub fn finalize(&mut self) -> Result<Vec<u8, ECC_BYTE_COUNT_STORE>, ()> {
        if self.scratch_space.len() == 0 {
            return Err(());
        }

        let mut rounds = self.generator.len();
        if self.scratch_space.len() < self.generator.len() {
            rounds = self.scratch_space.len();
            unsafe { self.scratch_space.resize(self.generator.len(), 0).unwrap_unchecked() };
        }

        for _ in 0..rounds {
            self.run_encoding_round();
            self.scratch_space.rotate_left(1);
            self.scratch_space[self.generator.len() - 1] = 0;
        }
        let mut out = self.scratch_space.clone();
        out.truncate(self.generator.len() - 1);
        self.reset();
        Ok(out)
    }

    pub fn reset(&mut self) {
        self.scratch_space.clear();
        self.bytes_processed = 0;
    }

    fn run_encoding_round(&mut self) {
        let coef = unsafe { self.scratch_space.get_unchecked(0) };
        if *coef != 0 {
            let lcoef = gf::LOG[*coef as usize] as usize;
            for j in 1..self.generator.len() {
                let scratch_var: &mut u8 = unsafe { &mut self.scratch_space.get_unchecked_mut(j) };
                let lgen_var = *unsafe { self.lgenerator.get_unchecked(j) };
                *scratch_var ^= gf::EXP[(lcoef + lgen_var as usize)];
            }
        }
    }
}

fn generator_poly<const MAX_LEN: usize>(ecclen: usize) -> [u8; MAX_LEN] {
    let mut gen = polynom![1];
    let mut mm = [1, 0];
    let mut i = 0;
    while i < ecclen {
        mm[1] = gf::pow(2, i as i32);
        gen = gen.mul(&mm);
        i += 1;
    }
    gen[..].try_into().unwrap()
}

pub const ENCODE_GEN_2_ECC_BYTES: [u8; 3] = [1, 3, 2];
pub const ENCODE_GEN_4_ECC_BYTES: [u8; 5] = [1, 15, 54, 120, 64];
pub const ENCODE_GEN_8_ECC_BYTES: [u8; 9] = [1, 255, 11, 81, 54, 239, 173, 200, 24];
pub const ENCODE_GEN_16_ECC_BYTES: [u8; 17] = [1, 59, 13, 104, 189, 68, 209, 30, 8, 163, 65, 41, 229, 98, 50, 36, 59];


#[cfg(test)]
mod tests {
    use std::vec::Vec;

    #[test]
    fn generator_poly() {
        assert_eq!([1, 3, 2], super::generator_poly(2));
        assert_eq!([1, 15, 54, 120, 64], super::generator_poly(4));
        assert_eq!([1, 255, 11, 81, 54, 239, 173, 200, 24], super::generator_poly(8));
        assert_eq!([1, 59, 13, 104, 189, 68, 209, 30, 8, 163, 65, 41, 229, 98, 50, 36, 59], super::generator_poly(16));
        assert_eq!([1, 116, 64, 52, 174, 54, 126, 16, 194, 162, 33, 33, 157, 176, 197, 225, 12,
                      59, 55, 253, 228, 148, 47, 179, 185, 24, 138, 253, 20, 142, 55, 172, 88],
            super::generator_poly(32)
        );
        assert_eq!([1, 193, 10, 255, 58, 128, 183, 115, 140, 153, 147, 91, 197, 219, 221, 220,
                      142, 28, 120, 21, 164, 147, 6, 204, 40, 230, 182, 14, 121, 48, 143, 77,
                      228, 81, 85, 43, 162, 16, 195, 163, 35, 149, 154, 35, 132, 100, 100, 51,
                      176, 11, 161, 134, 208, 132, 244, 176, 192, 221, 232, 171, 125, 155, 228,
                      242, 245],
            super::generator_poly(64)
        );
    }

    #[test]
    fn check_const_generators() {
        assert_eq!(super::ENCODE_GEN_2_ECC_BYTES, super::generator_poly::<3>(2));
        assert_eq!(super::ENCODE_GEN_4_ECC_BYTES, super::generator_poly::<5>(4));
        assert_eq!(super::ENCODE_GEN_8_ECC_BYTES, super::generator_poly::<9>(8));
        assert_eq!(super::ENCODE_GEN_16_ECC_BYTES, super::generator_poly::<17>(16));
    }

    #[test]
    fn encode() {
        let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
            22, 23, 24, 25, 26, 27, 28, 29];
        let ecc = [99, 26, 219, 193, 9, 94, 186, 143];

        let mut encoder = super::Encoder::<9>::new(ecc.len());
        let encoded = encoder.encode(&data[..]);

        assert_eq!(ecc, encoded);

        let mut encoder = super::Encoder::<9>::new_with_precomputed_generator(&super::ENCODE_GEN_8_ECC_BYTES);
        let encoded = encoder.encode(&data[..]);

        assert_eq!(ecc, encoded);
    }

    #[test]
    fn encode_shorter_than_ecc_message() {
        let data = [0, 1, 2, 3, 4];
        let ecc = [44, 157, 28, 43, 61, 248, 104, 250, 152, 77];

        let mut encoder = super::Encoder::<11>::new(ecc.len());
        let encoded = encoder.encode(&data[..]);

        assert_eq!(ecc, encoded);
    }

    #[test]
    fn encode_large() {
        let mut data = [0; 512];
        for i in 0..512 {
            data[i] = i as u8;
        }
        let expected = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99,
            100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199,
            200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246, 247, 248, 249,
            250, 251, 252, 62, 194, 253, 254, 255,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99,
            100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 197, 198, 199,
            200, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227, 228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244,
            245, 246, 247, 248, 249, 216, 37, 250, 251, 252, 253, 254, 255, 251, 250
        ];
        let mut result = Vec::new();

        let mut encoder = super::Encoder::<3>::new(2);
        for i in 0..512 {
            let encoded = encoder.encode_single(data[i]);
            result.extend_from_slice(&encoded[..]);
        }
        if let Ok(ecc) = encoder.finalize() {
            result.extend_from_slice(&ecc[..]);
        }

        assert_eq!(expected, *result);
    }
}
