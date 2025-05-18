#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]
#![allow(unsafe_op_in_unsafe_fn)]

pub mod vma {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}
