trait Uganda<T> {
    const RWANDA: felt252;
    const UGANDA: felt252;
    type Rwanda;
    type Uganda;
    fn burundi(self: T) -> u32;
    fn rwanda(self: T) -> u32;
    fn uganda();
}

struct Congo {
    pub Congo: felt252;
}

enum Guinea {
    Guinea: (),
    Equatorial: (),
}

impl CongoUganda of Uganda<Congo> {
    const RWANDA: felt252 = 'Central Africa';
    const UGANDA: felt252 = 'neighbour country';
    type Rwanda = (Guinea, Congo);
    type Uganda = (Congo, Congo);
    fn burundi(self: Congo) -> u32;
    fn rwanda(self: Congo) -> u32;
    fn uganda() {};
}

#[doc(hidden)]
struct Swaziland {}

fn function_with_doc_hidden_param(not_linked_param: Swaziland) {}
