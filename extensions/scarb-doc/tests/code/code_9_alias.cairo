
struct Point<T> {
    x: T,
    y: T,
}

type GenericPoint = Point<felt252>;

struct PointSimple {
    x: felt252,
    y: felt252,
}
type IntegerPointSimple = PointSimple;

type IntegerPointMissingGeneric = Point;

struct PointMultipleGenerics<T, U> {
    x: T,
    y: U,
}

type IntegerPointMultipleGeneric = PointMultipleGenerics<felt252, felt252>;

type AliasAlias = IntegerPointMissingGeneric;
