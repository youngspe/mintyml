use crate::ast::Space;

gramma::define_token!(
    #[pattern(matcher = {
        repeat(1.., char("]`~!@#$%^&)_+-=,./\"'?:;") + char('>') | char("}]")).simple()
    })]
    pub struct AnyClose;
);

gramma::define_rule!(
    pub struct UnmatchedClose {
        #[transform(prefer_short, for_each<ignore_before<Space>>)]
        pub close: Vec<AnyClose>,
    }
);
