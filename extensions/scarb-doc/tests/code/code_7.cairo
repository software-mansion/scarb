mod A {
    #[doc(group: 'test group 3')]
    /// Lorem ipsum dolor sit amet
    fn one() {}

    fn two() {}

    #[doc(group: 'test group 3')]
    struct Three {}

    #[doc(group: 'test group 4')]
    mod B {
            #[doc(group: 'this is visible only within the scope')]
            fn four() {}

            fn five() {}

            #[doc(group: 'this is visible only within the scope')]
            struct Six {}
    }
}

#[doc(group: 'test group 1')]
mod C {
    fn test_fn_present_in_group_documentation() {}
}
