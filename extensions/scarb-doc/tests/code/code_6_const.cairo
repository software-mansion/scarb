struct AnyStruct {
    a: u256,
    b: u32,
}

enum AnyEnum {
    A: felt252,
    B: (usize, u256),
}

const LITERAL_INT_CONST: u32 = 3600;
const STRUCT_INSTANCE_CONST: AnyStruct = AnyStruct { a: 0, b: 1 };
const ENUM_INSTANCE_CONST: AnyEnum = AnyEnum::A('any enum');
const BOOL_FIXED_SIZE_ARRAY_CONST: [bool; 2] = [true, false];
const ONE_HOUR_IN_SECONDS_EVAL_CONST: u32 = consteval_int!(60 * 60);
const TUPLE_CONST: (u32, u64, bool) = (10, 20, true);
const ARRAY_CONST: [u64; 5] = [1, 2, 3, 4, 5];
const EVAL_INT_CONST: felt252 = 2+2;
const BOOL_CONST: bool = true;
