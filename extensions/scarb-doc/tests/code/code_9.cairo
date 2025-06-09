pub use foreign_package::foreign_top_level_function;
pub use foreign_package::foreign_module::foreign_nested_module::foreign_nested_module_function;
pub use foreign_package::foreign_module::foreign_inner_mod_with_group;
pub use foreign_package::mod_with_group_item;
pub use same_parent_mod::nested_same_parent_mod::internal_reeksport;

fn top_level_function() {}

#[doc(group: 'test group')]
mod same_parent_mod {
    pub(crate) mod nested_same_parent_mod {
        #[doc(group: 'test group')]
        pub(crate) fn internal_reeksport() {}
    }
}

pub mod no_guarantee_of_uniqueness_in_all_pub_uses {
    // Group does guarantee uniqueness.
    pub use foreign_package::foreign_top_level_function;
}
