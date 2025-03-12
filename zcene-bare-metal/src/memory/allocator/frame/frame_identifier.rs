use core::range::Step;

////////////////////////////////////////////////////////////////////////////////////////////////////

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct FrameIdentifier(usize);

impl Step for FrameIdentifier {
    fn steps_between(start: &Self, end: &Self) -> (usize, Option<usize>) {
        <usize as Step>::steps_between(&start.0, &end.0)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        <usize as Step>::forward_checked(start.0, count).map(Self::new)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        <usize as Step>::backward_checked(start.0, count).map(Self::new)
    }

    fn forward(start: Self, count: usize) -> Self {
        Self::new(<usize as Step>::forward(start.0, count))
    }

    unsafe fn forward_unchecked(start: Self, count: usize) -> Self {
        unsafe { Self::new(<usize as Step>::forward_unchecked(start.0, count)) }
    }

    fn backward(start: Self, count: usize) -> Self {
        Self::new(<usize as Step>::backward(start.0, count))
    }

    unsafe fn backward_unchecked(start: Self, count: usize) -> Self {
        unsafe { Self::new(<usize as Step>::backward_unchecked(start.0, count)) }
    }
}

impl FrameIdentifier {
    pub fn new(value: usize) -> Self {
        Self(value)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}
