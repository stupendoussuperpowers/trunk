use std::process::Command;

fn get_base() -> String {
    "".to_string()
}

struct Kelvin {
    base: String,
}

impl Kelvin {
    fn new() -> Self {
        Kelvin { base: get_base() }
    }

    // fn read_output() -> Self {
    //     Command::new(program)
    // }
}

#[test]
fn it_works() {
    assert_eq!(4, 2 + 2);
}

// Test for -n

// Test for -f

// Test for -s
