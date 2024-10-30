#[derive(Copy)]
pub struct Polynom<const N: usize> {
    array: [u8; N],
    length: usize,
    dirty: bool,
}

impl<const N: usize> Polynom<N> {
    #[inline]
    pub const fn new() -> Self {
        Polynom {
            array: [0; N],
            length: 0,
            dirty: false,
        }
    }

    #[inline]
    pub const fn with_length(len: usize) -> Self {
        let mut p = Polynom::new();
        p.length = len;
        p
    }

    #[inline]
    pub fn set_length(&mut self, new_len: usize) {
        let old_len = self.len();
        self.length = new_len;
        
        if self.dirty && new_len > old_len {
            for x in self.iter_mut().skip(old_len)
                                    .take(new_len - old_len) 
            {
                *x = 0;
            }
        } else if new_len < old_len {
            self.dirty = true;
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    #[inline]
    pub fn reverse(mut self) -> Self {
        (*self).reverse();
        self
    }

    #[inline]
    pub fn push(&mut self, x: u8) {
        self.array[self.length] = x;
        self.length += 1;
    }

    pub fn get_mut(&mut self, index: usize) -> &mut u8 {
        &mut self.array[index]
    }
}

impl<const N: usize> Clone for Polynom<N> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<const N: usize> Default for Polynom<N> {
    fn default() -> Self {
        Self::new()
    }
}

use core::ops::Deref;
impl<const N: usize> Deref for Polynom<N> {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &Self::Target {
        let len = self.len();
        &self.array[0..len]
    }
}

use core::ops::DerefMut;
impl<const N: usize> DerefMut for Polynom<N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.len();
        &mut self.array[0..len]
    }
}

impl<'a, const N: usize> From<&'a [u8]> for Polynom<N> {
    #[inline]
    fn from(slice: &'a [u8]) -> Self {
        debug_assert!(slice.len() <= ::POLYNOMIAL_MAX_LENGTH);
        let mut poly = Polynom::with_length(slice.len());
        poly[..].copy_from_slice(slice);
        poly
    }
}

use core::fmt;
impl<const N: usize> fmt::Debug for Polynom<N> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{:?}", &self[..])
    }
}

#[cfg(test)]
mod tests {
    use gf::poly::Polynom;

    #[test]
    fn push() {
        let mut poly = Polynom::<10>::new();
        for i in 0..10 {
            poly.push(i);
            for j in 0..(i as usize) {
                assert!(poly[j] == j as u8);
            }
        }
    }

    #[test]
    fn reverse() {
        let poly = Polynom::<6>::from(&[5, 4, 3, 2, 1, 0][..]);
        for (i, x) in poly.reverse().iter().enumerate() {
            assert_eq!(i, *x as usize);
        }
    }

    #[test]
    fn set_length() {
        let mut poly = Polynom::<8>::from(&[1; 8][..]);
        poly.set_length(2);
        poly.set_length(6);

        for i in 0..2 {
            assert_eq!(poly.array[i], 1);
        }

        for i in 2..6 {
            assert_eq!(poly.array[i], 0);
        }
    }
}
