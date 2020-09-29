use std::{
    cmp::{Ord, Ordering},
    iter::Iterator,
};

pub trait SkipIterator: Iterator
where
    Self::Item: Ord + Copy,
{
    fn lower_bound_next(&mut self, min_id: Self::Item) -> Option<Self::Item> {
        while let Some(id) = self.next() {
            if id >= min_id {
                return Some(id);
            }
        }
        None
    }
}

// -------------------- leaf --------------------
struct ShortLeaf<'a, T: Ord + Copy> {
    slice: &'a [T],
}

pub fn short_leaf<'a>(slice: &'a [u32]) -> impl SkipIterator<Item = u32> + 'a {
    //impl SkipIterator + 'a {
    ShortLeaf { slice }
}

impl<'a, T: Ord + Copy> Iterator for ShortLeaf<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if let Some((head, tail)) = self.slice.split_first() {
            self.slice = tail;
            Some(*head)
        } else {
            None
        }
    }
}
impl<'a> SkipIterator for ShortLeaf<'a, u32> {}
// -------------------- leaf --------------------
struct Leaf<'a, T: Ord + Copy> {
    slice: &'a [T],
}

pub fn leaf<'a>(slice: &'a [u32]) -> impl SkipIterator<Item = u32> + 'a {
    //impl SkipIterator + 'a {
    Leaf { slice }
}

impl<'a, T: Ord + Copy> Iterator for Leaf<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if let Some((head, tail)) = self.slice.split_first() {
            self.slice = tail;
            Some(*head)
        } else {
            None
        }
    }
}

impl<'a> SkipIterator for Leaf<'a, u32> {
    fn lower_bound_next(&mut self, min_id: u32) -> Option<Self::Item> {
        match self.slice.binary_search(&min_id) {
            Ok(index) => {
                self.slice = &self.slice[index + 1..];
                Some(min_id)
            }
            Err(index) => {
                if index >= self.slice.len() {
                    self.slice = &[];
                    None
                } else {
                    let ret = self.slice[index];
                    self.slice = &self.slice[index + 1..];
                    Some(ret)
                }
            }
        }
        /*
        if self.slice.is_empty() {
            return None;
        }
        let mut lo = 0;
        let mut lo_id = self.slice[lo];
        if lo_id >= min_id {
            self.slice = &self.slice[lo + 1..];
            return Some(lo_id);
        }
        let mut hi = self.slice.len() - 1;
        let mut hi_id = self.slice[hi];
        if hi_id < min_id {
            self.slice = &self.slice[hi + 1..];
            return None;
        }
        while hi > lo {
            let mid = lo + (min_id - lo_id) as usize * (hi - lo) / (hi_id - lo_id) as usize;
            let mut mid_id: u32 = self.slice[mid];
            let stop = match mid_id.cmp(&min_id) {
                Ordering::Greater => {
                    hi = mid - 1;
                    hi_id = self.slice[hi];
                    if hi_id < min_id {
                        Some((mid, mid_id))
                    } else {
                        None
                    }
                }
                Ordering::Less => {
                    lo = mid + 1;
                    lo_id = self.slice[lo];
                    if lo_id >= min_id {
                        Some((lo, lo_id))
                    } else {
                        None
                    }
                }
                Ordering::Equal => Some((mid, mid_id)),
            };
            if let Some((index, id)) = stop {
                assert!(id >= min_id);
                self.slice = &self.slice[index + 1..];
                return Some(id);
            }
        }
        let ret: u32 = self.slice[lo as usize];
        self.slice = &self.slice[lo as usize + 1..];
        Some(ret)
        */
    }
}

/*
impl<'a, T: Ord + Copy> SkipIterator for Leaf<'a, T> {
    fn lower_bound_next(&mut self, min_id: T) -> Option<Self::Item> {
        match self.slice.binary_search(&min_id) {
            Ok(index) => {
                self.slice = &self.slice[index + 1..];
                Some(min_id)
            }
            Err(index) => {
                if index >= self.slice.len() {
                    self.slice = &[];
                    None
                } else {
                    let ret = self.slice[index];
                    self.slice = &self.slice[index + 1..];
                    Some(ret)
                }
            }
        }
    }
}*/

// -------------------- and --------------------
struct And<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>> {
    a: A,
    b: B,
}

pub fn and<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>>(
    a: A,
    b: B,
) -> impl SkipIterator<Item = T> {
    And { a, b }
}

impl<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>> And<T, A, B> {
    fn find_agreement(&mut self, mut target: T) -> Option<T> {
        while let Some(other) = self.b.lower_bound_next(target) {
            if other == target {
                return Some(target);
            }
            assert!(other > target);
            target = other;
            if let Some(other) = self.a.lower_bound_next(target) {
                if other == target {
                    return Some(target);
                }
                assert!(other > target);
                target = other;
            } else {
                break;
            }
        }
        None
    }
}

impl<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>> SkipIterator
    for And<T, A, B>
{
    fn lower_bound_next(&mut self, min_id: T) -> Option<Self::Item> {
        if let Some(target) = self.a.lower_bound_next(min_id) {
            self.find_agreement(target)
        } else {
            None
        }
    }
}

impl<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>> Iterator
    for And<T, A, B>
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(target) = self.a.next() {
            self.find_agreement(target)
        } else {
            None
        }
    }
}

// -------------------- diff --------------------
struct Difference<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>> {
    a: A,
    b: Option<B>,
    next_excluded: Option<T>,
}

impl<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>> Difference<T, A, B> {
    fn should_skip(&mut self, id: T) -> bool {
        if let &mut Some(ref mut b) = &mut self.b {
            if self.next_excluded.is_none() || self.next_excluded.unwrap() < id {
                if let Some(excluded) = b.lower_bound_next(id) {
                    if excluded == id {
                        self.next_excluded = None;
                        true
                    } else {
                        self.next_excluded = Some(excluded);
                        false
                    }
                } else {
                    self.b = None;
                    false
                }
            } else if self.next_excluded.unwrap() == id {
                self.next_excluded = None;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

pub fn diff<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>>(
    a: A,
    b: B,
) -> impl SkipIterator<Item = T> {
    Difference {
        a,
        b: Some(b),          // unset when b exhausted
        next_excluded: None, // retains next to exclude
    }
}

impl<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>> Iterator
    for Difference<T, A, B>
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.a.next() {
            if !self.should_skip(id) {
                return Some(id);
            }
        }
        None
    }
}

impl<T: Ord + Copy, A: SkipIterator<Item = T>, B: SkipIterator<Item = T>> SkipIterator
    for Difference<T, A, B>
{
    fn lower_bound_next(&mut self, min_id: Self::Item) -> Option<Self::Item> {
        if let Some(id) = self.a.lower_bound_next(min_id) {
            if self.should_skip(id) {
                self.next()
            } else {
                Some(id)
            }
        } else {
            None
        }
    }
}

struct Filter<T: Ord + Copy, F: FnMut(T) -> bool, Base: SkipIterator<Item = T>> {
    base: Base,
    filter: F,
}

pub fn filter_<T: Ord + Copy, F: FnMut(T) -> bool, Base: SkipIterator<Item = T>>(
    base: Base,
    filter: F,
) -> impl SkipIterator<Item = T> {
    Filter { base, filter }
}

impl<T: Ord + Copy, F: FnMut(T) -> bool, Base: SkipIterator<Item = T>> Iterator
    for Filter<T, F, Base>
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.base.next() {
            if (self.filter)(id) {
                return Some(id);
            }
        }
        None
    }
}

impl<T: Ord + Copy, F: FnMut(T) -> bool, Base: SkipIterator<Item = T>> SkipIterator
    for Filter<T, F, Base>
{
    fn lower_bound_next(&mut self, min_id: Self::Item) -> Option<Self::Item> {
        if let Some(id) = self.base.lower_bound_next(min_id) {
            if !(self.filter)(id) {
                self.next()
            } else {
                Some(id)
            }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test_skip_iterators {
    use super::*;

    #[test]
    fn test_leaf() {
        let mut it = leaf(&[2, 3, 5, 7, 11]);
        assert_eq!(it.next(), Some(2));
        assert_eq!(it.lower_bound_next(6), Some(7));
        assert_eq!(it.next(), Some(11));
    }

    #[test]
    fn test_and() {
        let and = and(
            leaf(&[2, 3, 5, 7, 11, 13, 17]),
            leaf(&[6, 7, 8, 9, 10, 11, 12]),
        );
        assert_eq!(vec![7, 11], and.collect::<Vec<_>>());
    }

    #[test]
    fn test_diff() {
        assert_eq!(
            vec![2, 3, 5, 13, 17],
            diff(
                leaf(&[2, 3, 5, 7, 11, 13, 17]),
                leaf(&[6, 7, 8, 9, 10, 11, 12]),
            )
            .collect::<Vec<_>>()
        );
        assert!(diff(leaf(&[2, 3, 5]), leaf(&[1, 2, 3, 4, 5, 6]),)
            .next()
            .is_none());
        assert!(diff(leaf(&[2]), diff(leaf(&[1, 2]), leaf(&[])))
            .next()
            .is_none());
    }
}
