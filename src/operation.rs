//! Zero-sized operation markers.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Add;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sub;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Mul;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Div;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Rem;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Neg;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sqrt;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ln;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Log2;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Ln1p;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Exp;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExpM1;
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PowI;
