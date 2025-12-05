#[doc(group: "visible")]
pub fn top_level_fn_visible_in_group() {}

mod inner_module {
    #[doc(group: "visible")]
    pub fn inner_function_visible_in_group() {
    // this should be visible in the group and parent pub uses
    }

    pub fn invisible_function() {}

    pub fn visible_in_reeksports() {}

    #[doc(group: "invisible group")]
    pub fn this_should_not_be_documented() {}

    pub struct LinkedStruct {}

    pub struct TestMembersVisibility {
        invisible_field: felt252,
        invisible_field2_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_long_one: felt252,
        pub visible_field: LinkedStruct,
        pub visible_field2_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_very_long_one: LinkedStruct,
        invisible_field3: felt252,
    }

    #[doc(group: "visible")]
    pub struct TestMembersVisibility2 {
        no_fields_should_be_documented: LinkedStruct,
    }


}

mod macro_module {
    pub macro macro_definition {
        ($name:ident) => {
            fn $name() {
                println!(name);
            }
        };
    }
}

pub use inner_module::visible_in_reeksports;
pub use inner_module::inner_function_visible_in_group;
pub use macro_module::macro_definition;
pub use inner_module::{LinkedStruct, TestMembersVisibility, TestMembersVisibility2};
