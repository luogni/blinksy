pub trait Pattern1d<const N: usize> {
    type Params;
    type Layout;
    type Color;

    fn new(params: Self::Params, layout: Self::Layout) -> Self;
    fn tick(time_in_ms: u64) -> [Self::Color; N];
}
