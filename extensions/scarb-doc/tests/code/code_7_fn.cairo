struct AnyStruct {
    a: u256,
    b: u32,
}

pub(crate) fn struct_function(arg1: AnyStruct,) -> (u256, u32) {
    (arg1.a, arg1.b)
}

fn multiple_args_function(arg1: u8, arg2: felt252,  mut arg3: AnyStruct) -> AnyStruct {
    arg3
}

fn tuple_function(pair: (i32, i32)) -> i32 {
    let (a,b) = pair;
    a+b
}

pub fn nested_tuple_function(pair: (i32, (i32, i32), [i32; 5])) -> i32 {
    let (a, (b, c), r) = pair;
    a+b
}

pub fn tuples_array_function(array: [(bool, bool, bool); 2]) -> [(bool, bool, bool); 2] {
    array
}
