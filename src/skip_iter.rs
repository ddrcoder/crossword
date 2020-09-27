use std::iter::Iterator;

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
struct Leaf<'a, T: Ord + Copy> {
    slice: &'a [T],
}

pub fn leaf<'a, T: Ord + Copy>(slice: &'a [T]) -> impl SkipIterator<Item = T> + 'a {
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

impl<'a, T: Ord + Copy> SkipIterator for Leaf<'a, T> {}

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
        let diff = diff(
            leaf(&[2, 3, 5, 7, 11, 13, 17]),
            leaf(&[6, 7, 8, 9, 10, 11, 12]),
        );
        assert_eq!(vec![2, 3, 5, 13, 17], diff.collect::<Vec<_>>());
    }
}
