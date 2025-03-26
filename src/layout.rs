use heapless::Vec;

pub struct Layout1d {
    pub length: usize,
}

pub enum Shape2d {}

pub struct Layout2d<const N: usize>(Vec<Shape2d, N>);

pub enum Shape3d {}

pub struct Layout3d<const N: usize>(Vec<Shape3d, N>);
