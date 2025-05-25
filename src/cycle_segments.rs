use itertools::Itertools;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CycleSegment {
    start: usize,

    /// Exclusive
    end: usize,

    /// Length of the whole cycle of which this segment is a part.
    cycle_len: usize,
}

impl CycleSegment {
    /// Empty segment not allowed, doesn't make sense.
    pub fn new(start: usize, end: usize, cycle_len: usize) -> Self {
        assert!(end > start);
        Self {
            start,
            end,
            cycle_len,
        }
    }

    pub fn first(self) -> usize {
        self.start
    }

    pub fn last(self) -> usize {
        (self.end - 1) % self.cycle_len
    }

    pub fn wraps_around(self) -> bool {
        self.end > self.cycle_len
    }

    pub fn len(self) -> usize {
        self.end - self.start
    }

    /// Iterate over the indices contained in the segment.
    pub fn iter(self) -> impl DoubleEndedIterator<Item = usize> + Clone {
        (self.start..self.end).map(move |i| i % self.cycle_len)
    }
}

/// A step is an index `i` where `a[i - 1] != a[i]`. A segment is a range where the value is
/// constant.
/// Example:
/// Array:    [a, a, a, b, b, b, c, c, c, c, c, a, a]
/// Steps:            |        |              |
/// Segments:  -------][-------][-------------][----
/// The beginning and end of the array are in the same segment.
/// Steps at 3, 6, 11
/// Segments 3..6, 6...11, 11..3
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CycleSegments {
    /// For the special case that the cycle only has one segment steps is `[cycle.len()]`.
    steps: Vec<usize>,
    cycle_len: usize,
}

impl CycleSegments {
    /// See `from_iter`
    pub fn from_slice<T: Eq + Clone>(cycle: &[T]) -> Self {
        Self::from_iter(cycle.iter())
    }

    /// `cycle` cannot be empty.
    pub fn from_iter<T: Eq + Clone>(cycle: impl ExactSizeIterator<Item = T> + Clone) -> Self {
        let cycle_len = cycle.len();
        assert!(cycle_len > 0);

        let mut steps: Vec<usize> = cycle
            .enumerate()
            .circular_tuple_windows()
            .filter_map(|((_, value), (next_i, next_value))| {
                (value != next_value).then_some(next_i)
            })
            .collect();

        if steps.is_empty() {
            steps = vec![0];
        }

        Self { steps, cycle_len }
    }

    /// Returns the indices of the segment as a Range mod `self.cycle_len`
    pub fn atomic_segment(&self, i: usize) -> CycleSegment {
        self.segment(i, 1)
    }

    pub fn segment(&self, i: usize, count: usize) -> CycleSegment {
        assert!(count > 0);
        let start_step = self.steps[i];
        let mut stop_step = self.steps[(i + count) % self.steps.len()];
        if stop_step <= start_step {
            // In case the segment wraps around
            stop_step += self.cycle_len;
        }

        CycleSegment::new(start_step, stop_step, self.cycle_len)
    }

    /// Number of segments
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Iterate segments
    pub fn iter(&self) -> impl ExactSizeIterator<Item = CycleSegment> + Clone + '_ {
        (0..self.len()).map(|i| self.atomic_segment(i))
    }
}

#[cfg(test)]
mod test {
    use crate::cycle_segments::CycleSegments;
    use itertools::Itertools;

    fn check_segments(cycle_str: &str) {
        let cycle = cycle_str.as_bytes();
        let segments = CycleSegments::from_slice(cycle);

        // Check that sum of lengths equals cycle.len()
        let total_len: usize = segments.iter().map(|segment| segment.len()).sum();
        assert_eq!(total_len, cycle.len());

        // Check that cycle is constant on each segment
        for segment in segments.iter() {
            assert!(segment.iter().map(|i| cycle[i]).all_equal());
        }

        // Check that two neighboring segments have different values
        if segments.len() > 1 {
            for (segment, next_segment) in segments.iter().circular_tuple_windows() {
                assert_ne!(cycle[segment.first()], cycle[next_segment.first()]);
            }
        }
    }

    #[test]
    fn one_segment() {
        check_segments("aaaaaa");
        check_segments("aaaaaaa");
        check_segments("a");
    }

    #[test]
    fn two_segments() {
        check_segments("aaaaab");
        check_segments("baaaaa");
        check_segments("aabb");
    }

    #[test]
    fn two_segments_wrapping() {
        check_segments("aba");
        check_segments("aaaba");
    }

    #[test]
    fn three_segments() {
        check_segments("aabbcc");
        check_segments("abbccca");
    }
}
