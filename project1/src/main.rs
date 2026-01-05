//! Test crate

fn main() {
    println!("Project 1!");
}

fn add(a: i32, b: i32) -> i32 {
    let mut sum = 0;
    for _i in 0..a {
        sum += 1;
    }
    for _i in 0..b {
        sum += 1;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_simple() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn test_add_with_negative_number() {
        assert_eq!(add(5, -2), 3);
    }
}