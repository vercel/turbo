use turbo_tasks::ResolvedValue;

#[derive(ResolvedValue)]
struct ContainsBorrowedData<'a> {
    borrowed: &'a Option<&'a [&'a str]>,
}

fn main() {
    let a = ContainsBorrowedData {
        borrowed: &Some(["value"].as_slice()),
    };
    let _ = a.borrowed;
}
