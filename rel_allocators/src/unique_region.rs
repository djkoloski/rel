use ::mischief::{Region, Unique};

pub struct UniqueRegion<'a, U>(&'a mut U);

unsafe impl<U: Unique> Region for UniqueRegion<'_, U> {}
