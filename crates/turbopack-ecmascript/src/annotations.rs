use swc_core::{
    common::DUMMY_SP,
    ecma::ast::{Expr, KeyValueProp, ObjectLit, Prop, PropName, PropOrSpread},
};

pub fn with_chunking_type(chunking_type: &str) -> Box<ObjectLit> {
    with_clause(&[("chunking-type", chunking_type)])
}

pub fn with_transition(transition_name: &str) -> Box<ObjectLit> {
    with_clause(&[("transition", transition_name)])
}

pub fn with_clause<'a>(
    entries: impl IntoIterator<Item = &'a (&'a str, &'a str)>,
) -> Box<ObjectLit> {
    Box::new(ObjectLit {
        span: DUMMY_SP,
        props: entries.into_iter().map(|(k, v)| with_prop(k, v)).collect(),
    })
}

fn with_prop(key: &str, value: &str) -> PropOrSpread {
    PropOrSpread::Prop(Box::new(Prop::KeyValue(KeyValueProp {
        key: PropName::Str(key.into()),
        value: Box::new(Expr::Lit(value.into())),
    })))
}
