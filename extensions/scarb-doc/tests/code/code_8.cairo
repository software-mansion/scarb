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

}

pub use inner_module::visible_in_reeksports;
pub use inner_module::inner_function_visible_in_group;
