use std::{fmt::*, *};

///Output dbg!(expr) only in cfg!(test) || cfg!(debug_assertions) context
pub fn tdbg<T: Debug>(expr: T) -> T {
    if cfg!(test) || cfg!(debug_assertions) {
        dbg!(expr)
    } else {
        expr
    }
}

#[macro_export]
macro_rules! demo {
    ([$attr:meta] $pub:vis & $lt:lifetime $name:ident:$type:ty = $l:literal | $e:expr , $s:stmt ; $pat:pat => $b:block | $p:path | $i:item | $t:tt ) => {};
}

macro_rules! impl_ref_elements {
    () => {};
    ($T0:ident $($T:ident)*) => {
        impl<$T0, $($T,)*> RefElements for ($T0,$($T,)*) {
            type Refs<'a> = (&'a $T0, $(&'a $T,)*) where Self:'a;
            fn ref_elements(&self)->Self::Refs<'_> {
                let &(ref $T0,$(ref $T,)*) = self;
                ($T0,$($T,)*)
            }
        }
        impl_ref_elements!{$($T)*}
    }
}
