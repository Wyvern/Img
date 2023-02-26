use std::{fmt::*, *};

///Output dbg!(expr) only in cfg!(test) || cfg!(debug_assertions) context
pub fn test_dbg<T: Debug>(expr: T) -> T {
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
