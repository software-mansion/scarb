#[doc(group: 'test group')]
pub fn foreign_top_level_function(){}

pub mod foreign_module {
    pub mod foreign_nested_module {
        pub fn foreign_nested_module_function() {}
        fn this_should_not_be_visible() {}
    }
    fn this_should_not_be_visible_either() {}

    #[doc(group: 'visible')]
    pub mod foreign_inner_mod_with_group {
        //! This should be in nav bar in Groups but documented in Reeksports of target_crate
        pub fn function_of_reeksported_group_mod(){}
    }
}

pub mod mod_with_group_item {
    pub mod ensure_it_is_documented_in_mod {}

    #[doc(group: 'visible')]
    pub mod ensure_it_is_documented_in_group {}

    fn not_documented() {}

}
