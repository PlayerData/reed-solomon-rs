use core::convert::TryInto;
use super::gf::poly_math::*;
use super::gf;
use heapless::Vec;

/// Reed-Solomon BCH encoder
#[derive(Debug)]
pub struct Encoder<const ECC_BYTE_COUNT_STORE: usize> {
    generator: Vec<u8, ECC_BYTE_COUNT_STORE>,
    lgenerator: Vec<u8, ECC_BYTE_COUNT_STORE>,
}

impl<const ECC_BYTE_COUNT_STORE: usize> Encoder<ECC_BYTE_COUNT_STORE> {
    fn make_lgenerator(generator: &Vec<u8, ECC_BYTE_COUNT_STORE>) -> Vec<u8, ECC_BYTE_COUNT_STORE> {
        let mut lgen = Vec::<u8, ECC_BYTE_COUNT_STORE>::new();
        for gen_i in generator.iter() {
            unsafe {lgen.push_unchecked(gf::LOG[*gen_i as usize])};
        }
        lgen
    }

    /// Constructs a new `Encoder` and calculates generator polynomial of given `ecc_len`.
    ///
    /// # Example
    /// ```rust
    /// use reed_solomon::Encoder;
    ///
    /// let encoder = Encoder::new(8);
    /// ```
    pub fn new(ecc_len: usize) -> Self {
        let generator = generator_poly(ecc_len);

        Self {
            lgenerator: Self::make_lgenerator(&generator),
            generator,
        }
    }


    // generator_poly should be called to produce an array to be passed into this function
    // The array should be ecc_len + 1 bytes long
    pub fn new_with_precomputed_generator(generator: &[u8]) -> Result<Self, ()> {
        let generator = generator.try_into()?;
        Ok(Self {
            lgenerator: Self::make_lgenerator(&generator),
            generator,
        })
    }

    /// Encodes passed `&[u8]` slice and returns `Buffer` with result and `ecc` offset.
    ///
    /// # Example
    /// ```rust
    /// use reed_solomon::Encoder;
    ///
    /// let data = "Hello World".as_bytes();
    /// let encoder = Encoder::new(8);
    ///
    /// let encoded = encoder.encode(&data);
    ///
    /// println!("whole: {:?}", &encoded[..]);
    /// println!("data:  {:?}", encoded.data());
    /// println!("ecc:   {:?}", encoded.ecc());
    /// ```
    pub fn encode(&self, data: &[u8]) -> Vec<u8, ECC_BYTE_COUNT_STORE> {
        let mut scratch_space: [u8; ECC_BYTE_COUNT_STORE] = [0; ECC_BYTE_COUNT_STORE];
        scratch_space[1..].copy_from_slice(&data[..(ECC_BYTE_COUNT_STORE - 1)]);

        for i in 0..data.len() {
            scratch_space.rotate_left(1);
            if i + ECC_BYTE_COUNT_STORE -1 < data.len() {
                scratch_space[ECC_BYTE_COUNT_STORE - 1] = data[i + ECC_BYTE_COUNT_STORE -1];
            } else {
                scratch_space[ECC_BYTE_COUNT_STORE - 1] = 0;
            }

            let coef = unsafe {scratch_space.get_unchecked(0)};
            if *coef != 0 {
                let lcoef = gf::LOG[*coef as usize] as usize;
                for j in 1..self.generator.len() {
                    let scratch_var: &mut u8 = unsafe { &mut scratch_space.get_unchecked_mut(j) };
                    let lgen_var = *unsafe {self.lgenerator.get_unchecked(j)};
                    *scratch_var ^= gf::EXP[(lcoef + lgen_var as usize)];
                }
            }
        }

        scratch_space.rotate_left(1);
        Vec::from_slice(&scratch_space[..(self.generator.len() - 1)]).unwrap()
    }
}

fn generator_poly<const MAX_LEN: usize>(ecclen: usize) -> Vec<u8, MAX_LEN> {
    let mut gen = polynom![1];
    let mut mm = [1, 0];
    let mut i = 0;
    while i < ecclen {
        mm[1] = gf::pow(2, i as i32);
        gen = gen.mul(&mm);
        i += 1;
    }
    Vec::from_slice(&gen[..]).unwrap()
}

pub const ENCODE_GEN_2_ECC_BYTES: [u8; 3] = [1, 3, 2];
pub const ENCODE_GEN_4_ECC_BYTES: [u8; 5] = [1, 15, 54, 120, 64];
pub const ENCODE_GEN_8_ECC_BYTES: [u8; 9] = [1, 255, 11, 81, 54, 239, 173, 200, 24];
pub const ENCODE_GEN_16_ECC_BYTES: [u8; 17] = [1, 59, 13, 104, 189, 68, 209, 30, 8, 163, 65, 41, 229, 98, 50, 36, 59];


#[cfg(test)]
mod tests {
    use crate::gf::poly::Polynom;

    #[test]
    fn generator_poly() {
        let answers =
            [Polynom::<65>::from(&[1, 3, 2][..]),
             Polynom::<65>::from(&[1, 15, 54, 120, 64][..]),
             Polynom::<65>::from(&[1, 255, 11, 81, 54, 239, 173, 200, 24][..]),
             Polynom::<65>::from(&[1, 59, 13, 104, 189, 68, 209, 30, 8, 163, 65, 41, 229, 98, 50, 36, 59][..]),
             Polynom::<65>::from(&[1, 116, 64, 52, 174, 54, 126, 16, 194, 162, 33, 33, 157, 176, 197, 225, 12,
                      59, 55, 253, 228, 148, 47, 179, 185, 24, 138, 253, 20, 142, 55, 172, 88][..]),
             Polynom::<65>::from(&[1, 193, 10, 255, 58, 128, 183, 115, 140, 153, 147, 91, 197, 219, 221, 220,
                      142, 28, 120, 21, 164, 147, 6, 204, 40, 230, 182, 14, 121, 48, 143, 77,
                      228, 81, 85, 43, 162, 16, 195, 163, 35, 149, 154, 35, 132, 100, 100, 51,
                      176, 11, 161, 134, 208, 132, 244, 176, 192, 221, 232, 171, 125, 155, 228,
                      242, 245][..])];

        let mut ecclen = 2;
        for i in 0..6 {
            assert_eq!(*answers[i], *super::generator_poly::<65>(ecclen));
            ecclen *= 2;
        }
    }

    #[test]
    fn check_const_generators() {
        assert_eq!(super::ENCODE_GEN_2_ECC_BYTES, *super::generator_poly::<3>(2));
        assert_eq!(super::ENCODE_GEN_4_ECC_BYTES, *super::generator_poly::<5>(4));
        assert_eq!(super::ENCODE_GEN_8_ECC_BYTES, *super::generator_poly::<9>(8));
        assert_eq!(super::ENCODE_GEN_16_ECC_BYTES, *super::generator_poly::<17>(16));
    }

    #[test]
    fn encode() {
        let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
                    22, 23, 24, 25, 26, 27, 28, 29];
        let ecc = [99, 26, 219, 193, 9, 94, 186, 143];

        let mut encoder = super::Encoder::<9>::new(ecc.len());
        let encoded = encoder.encode(&data[..]);

        assert_eq!(ecc, encoded);

        let const_encoder = super::Encoder::<9>::new_with_precomputed_generator(&super::ENCODE_GEN_8_ECC_BYTES).unwrap();
        let const_encoded = const_encoder.encode(&data[..]);

        assert_eq!(ecc, const_encoded);
    }

}
